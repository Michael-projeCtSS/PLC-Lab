//! Deterministic virtual industrial network with a behavioral Modbus TCP implementation.
//!
//! Model: one switched Ethernet segment with a fixed one-way latency. Delivery follows
//! IPv4 subnet rules (no route without a gateway, ARP resolution on the segment, address
//! conflicts), TCP connection setup is abstracted to round trips, and Modbus TCP
//! requests/responses follow the application protocol (function codes 03/04, exception
//! responses, zero-based register addressing, MBAP unit identifier). Timing is
//! educational, not a conformance or performance model.
//!
//! All events are processed in `(time, sequence)` order from an internal queue; the
//! engine interleaves them with world events and physics.

use lab_model::{
    Identifier,
    network::{
        DeviceRole, InterfaceConfig, Ipv4, ModbusDataType, ModbusReadFunction, ModbusTcpScan, NetworkDevice,
        NetworkInstallation, RegisterValue, UnitIdBehavior, WordOrder,
    },
};
use simulation_core::WorldTime;
use std::collections::BTreeMap;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod exception {
    pub const ILLEGAL_FUNCTION: u8 = 0x01;
    pub const ILLEGAL_DATA_ADDRESS: u8 = 0x02;
    pub const GATEWAY_TARGET_FAILED: u8 = 0x0B;
}

/// Reads physical quantities for server registers at the moment a request is served.
pub trait QuantitySource {
    fn quantity(&self, component: &str, name: &str) -> Option<i64>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScanStatus {
    /// No transaction completed yet.
    Idle,
    Ok,
    Timeout,
    NoRoute,
    ConnectionRefused,
    Exception(u8),
    NoInterface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkEventKind {
    RequestSent { function: u8, address: u16, quantity: u16, unit: u8 },
    ResponseReceived { round_trip_us: u64 },
    ExceptionReceived { code: u8 },
    Timeout,
    NoRoute,
    ArpUnresolved { ip: Ipv4 },
    ConnectionRefused,
    AddressConflict { ip: Ipv4 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkEvent {
    pub time: WorldTime,
    pub scan: usize,
    pub transaction: u16,
    pub peer: Ipv4,
    pub kind: NetworkEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanValue {
    Int(i16),
    Real(f32),
    /// The decoded binary32 is NaN or infinite and cannot enter PLC REAL memory.
    NotFinite,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScanView {
    pub id: Identifier,
    pub server: Ipv4,
    pub unit_id: u8,
    pub status: ScanStatus,
    pub connected: bool,
    pub requests: u64,
    pub responses: u64,
    pub timeouts: u64,
    pub exceptions: u64,
    pub last_round_trip_us: Option<u64>,
    pub registers: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceView {
    pub id: Identifier,
    pub label: String,
    pub ip: Ipv4,
    pub mask: Ipv4,
    pub link_up: bool,
    pub online: bool,
    pub modbus_server: bool,
    pub requests_served: u64,
    pub last_activity: Option<WorldTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Event {
    Poll { scan: usize },
    Arrive { scan: usize, tx: u16, device: usize },
    Refused { scan: usize, tx: u16 },
    Respond { scan: usize, tx: u16, result: Result<(), u8>, device: usize },
    Expire { scan: usize, tx: u16 },
}

#[derive(Debug, Clone)]
struct ScanState {
    config: ModbusTcpScan,
    status: ScanStatus,
    connected: bool,
    pending: Option<(u16, u64)>,
    next_tx: u16,
    registers: Vec<u16>,
    staged: Vec<u16>,
    requests: u64,
    responses: u64,
    timeouts: u64,
    exceptions: u64,
    last_round_trip: Option<u64>,
}

#[derive(Debug, Clone)]
struct DeviceState {
    def: NetworkDevice,
    ip: Ipv4,
    link_up: bool,
    online: bool,
    served: u64,
    last_activity: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct NetworkRuntime {
    latency: u64,
    plc_link_up: bool,
    interface: Option<InterfaceConfig>,
    devices: Vec<DeviceState>,
    scans: Vec<ScanState>,
    queue: BTreeMap<(u64, u64), Event>,
    sequence: u64,
    now: u64,
    events: Vec<NetworkEvent>,
    plc_activity: Option<u64>,
}

impl NetworkRuntime {
    pub fn new(installation: Option<&NetworkInstallation>) -> Self {
        let (latency, plc_link_up, devices) = installation.map_or((500, true, Vec::new()), |n| {
            (
                u64::from(n.latency_us).max(1),
                n.plc_link_up,
                n.devices
                    .iter()
                    .map(|d| DeviceState {
                        ip: d.interface.ip,
                        link_up: d.link_up,
                        online: true,
                        served: 0,
                        last_activity: None,
                        def: d.clone(),
                    })
                    .collect(),
            )
        });
        Self {
            latency,
            plc_link_up,
            interface: None,
            devices,
            scans: Vec::new(),
            queue: BTreeMap::new(),
            sequence: 0,
            now: 0,
            events: Vec::new(),
            plc_activity: None,
        }
    }

    /// Apply the station's configured interface and scan list (a download). Restarts scanning.
    pub fn configure(&mut self, interface: Option<InterfaceConfig>, scans: &[ModbusTcpScan]) {
        self.interface = interface;
        self.queue.clear();
        self.scans = scans
            .iter()
            .map(|s| ScanState {
                status: ScanStatus::Idle,
                connected: false,
                pending: None,
                next_tx: 1,
                registers: Vec::new(),
                staged: Vec::new(),
                requests: 0,
                responses: 0,
                timeouts: 0,
                exceptions: 0,
                last_round_trip: None,
                config: s.clone(),
            })
            .collect();
        for scan in 0..self.scans.len() {
            self.push(self.now, Event::Poll { scan });
        }
    }

    fn push(&mut self, time: u64, event: Event) {
        self.queue.insert((time, self.sequence), event);
        self.sequence += 1;
    }

    pub fn next_event_time(&self) -> Option<WorldTime> {
        self.queue.keys().next().map(|(t, _)| WorldTime::from_micros(*t))
    }

    pub fn drain_events(&mut self) -> Vec<NetworkEvent> {
        std::mem::take(&mut self.events)
    }

    fn device_index(&self, id: &Identifier) -> Option<usize> {
        self.devices.iter().position(|d| &d.def.id == id)
    }
    pub fn set_device_link(&mut self, id: &Identifier, up: bool) -> bool {
        self.device_index(id).map(|i| self.devices[i].link_up = up).is_some()
    }
    pub fn set_device_online(&mut self, id: &Identifier, online: bool) -> bool {
        self.device_index(id).map(|i| self.devices[i].online = online).is_some()
    }
    pub fn set_device_ip(&mut self, id: &Identifier, ip: Option<Ipv4>) -> bool {
        self.device_index(id)
            .map(|i| self.devices[i].ip = ip.unwrap_or(self.devices[i].def.interface.ip))
            .is_some()
    }
    pub fn set_plc_link(&mut self, up: bool) {
        self.plc_link_up = up;
    }

    /// Process the next internal event if it is due at or before `limit`.
    pub fn process_next(&mut self, limit: WorldTime, source: &dyn QuantitySource) -> bool {
        let Some((&(time, seq), _)) = self.queue.iter().next() else { return false };
        if time > limit.as_micros() {
            return false;
        }
        let event = self.queue.remove(&(time, seq)).expect("present");
        self.now = time;
        self.handle(event, source);
        true
    }

    /// Advance the network clock without events (the engine already processed all due events).
    pub fn advance_to(&mut self, time: WorldTime) {
        self.now = self.now.max(time.as_micros());
    }

    fn emit(&mut self, scan: usize, tx: u16, peer: Ipv4, kind: NetworkEventKind) {
        self.events.push(NetworkEvent { time: WorldTime::from_micros(self.now), scan, transaction: tx, peer, kind });
    }

    fn resolve(&mut self, scan: usize, tx: u16, ip: Ipv4) -> Option<usize> {
        let candidates: Vec<usize> = self
            .devices
            .iter()
            .enumerate()
            .filter(|(_, d)| d.ip == ip && d.link_up && d.online)
            .map(|(i, _)| i)
            .collect();
        match candidates.len() {
            0 => None,
            1 => Some(candidates[0]),
            n => {
                self.emit(scan, tx, ip, NetworkEventKind::AddressConflict { ip });
                // Competing ARP replies: the answering device alternates per transaction.
                Some(candidates[usize::from(tx) % n])
            }
        }
    }

    fn handle(&mut self, event: Event, source: &dyn QuantitySource) {
        let now = self.now;
        match event {
            Event::Poll { scan } => {
                let period = u64::from(self.scans[scan].config.period_ms) * 1_000;
                self.push(now + period, Event::Poll { scan });
                if self.scans[scan].pending.is_some() {
                    return;
                }
                let config = self.scans[scan].config.clone();
                let tx = self.scans[scan].next_tx;
                self.scans[scan].next_tx = tx.wrapping_add(1).max(1);
                let Some(interface) = self.interface.clone() else {
                    self.scans[scan].status = ScanStatus::NoInterface;
                    return;
                };
                self.scans[scan].requests += 1;
                self.plc_activity = Some(now);
                let timeout = u64::from(config.timeout_ms) * 1_000;
                if !interface.ip.same_subnet(config.server, interface.mask) && interface.gateway.is_none() {
                    self.scans[scan].status = ScanStatus::NoRoute;
                    self.scans[scan].connected = false;
                    self.emit(scan, tx, config.server, NetworkEventKind::NoRoute);
                    return;
                }
                self.scans[scan].pending = Some((tx, now));
                self.push(now + timeout, Event::Expire { scan, tx });
                self.emit(
                    scan,
                    tx,
                    config.server,
                    NetworkEventKind::RequestSent {
                        function: config.function.code(),
                        address: config.start_address,
                        quantity: config.quantity,
                        unit: config.unit_id,
                    },
                );
                if !self.plc_link_up {
                    return;
                }
                let next_hop = if interface.ip.same_subnet(config.server, interface.mask) {
                    config.server
                } else {
                    interface.gateway.unwrap_or(config.server)
                };
                // A gateway would forward to another subnet; no routers are modeled on the segment.
                let device = if next_hop == config.server { self.resolve(scan, tx, next_hop) } else { None };
                let Some(device) = device else {
                    self.emit(scan, tx, next_hop, NetworkEventKind::ArpUnresolved { ip: next_hop });
                    return;
                };
                let setup = if self.scans[scan].connected { 0 } else { 2 * self.latency };
                if matches!(self.devices[device].def.role, DeviceRole::Host) || config.port != 502 {
                    self.push(now + 2 * self.latency, Event::Refused { scan, tx });
                } else {
                    self.push(now + setup + self.latency, Event::Arrive { scan, tx, device });
                }
            }
            Event::Refused { scan, tx } => {
                if self.scans[scan].pending.map(|p| p.0) == Some(tx) {
                    let peer = self.scans[scan].config.server;
                    self.scans[scan].pending = None;
                    self.scans[scan].connected = false;
                    self.scans[scan].status = ScanStatus::ConnectionRefused;
                    self.emit(scan, tx, peer, NetworkEventKind::ConnectionRefused);
                }
            }
            Event::Arrive { scan, tx, device } => {
                self.scans[scan].connected = true;
                let Some(interface) = self.interface.clone() else { return };
                let d = &mut self.devices[device];
                d.last_activity = Some(now);
                if !d.link_up || !d.online {
                    return;
                }
                // The server must be able to route its reply back to the client.
                if !d.ip.same_subnet(interface.ip, d.def.interface.mask) && d.def.interface.gateway.is_none() {
                    return;
                }
                let DeviceRole::ModbusTcpServer { unit_ids, processing_us, holding_registers, input_registers } = &d.def.role else {
                    return;
                };
                let config = &self.scans[scan].config;
                let unit_ok = match unit_ids {
                    UnitIdBehavior::Ignore => true,
                    UnitIdBehavior::Accept { units } => units.contains(&config.unit_id),
                };
                let table = match config.function {
                    ModbusReadFunction::ReadHoldingRegisters => holding_registers,
                    ModbusReadFunction::ReadInputRegisters => input_registers,
                };
                let result = if !unit_ok {
                    Err(exception::GATEWAY_TARGET_FAILED)
                } else if table.is_empty() {
                    Err(exception::ILLEGAL_FUNCTION)
                } else {
                    let mut words = Vec::with_capacity(usize::from(config.quantity));
                    let mut ok = true;
                    for offset in 0..config.quantity {
                        let Some(address) = config.start_address.checked_add(offset) else {
                            ok = false;
                            break;
                        };
                        match register(table, address, source) {
                            Some(w) => words.push(w),
                            None => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if ok {
                        self.scans[scan].staged = words;
                        Ok(())
                    } else {
                        Err(exception::ILLEGAL_DATA_ADDRESS)
                    }
                };
                let processing = u64::from(*processing_us);
                d.served += 1;
                self.push(now + processing + self.latency, Event::Respond { scan, tx, result, device });
            }
            Event::Respond { scan, tx, result, device } => {
                let _ = device;
                let Some((pending, sent)) = self.scans[scan].pending else { return };
                if pending != tx {
                    return;
                }
                let peer = self.scans[scan].config.server;
                let state = &mut self.scans[scan];
                state.pending = None;
                state.last_round_trip = Some(now - sent);
                self.plc_activity = Some(now);
                match result {
                    Ok(()) => {
                        state.registers = std::mem::take(&mut state.staged);
                        state.status = ScanStatus::Ok;
                        state.responses += 1;
                        self.emit(scan, tx, peer, NetworkEventKind::ResponseReceived { round_trip_us: now - sent });
                    }
                    Err(code) => {
                        state.status = ScanStatus::Exception(code);
                        state.exceptions += 1;
                        self.emit(scan, tx, peer, NetworkEventKind::ExceptionReceived { code });
                    }
                }
            }
            Event::Expire { scan, tx } => {
                if self.scans[scan].pending.map(|p| p.0) == Some(tx) {
                    let peer = self.scans[scan].config.server;
                    let state = &mut self.scans[scan];
                    state.pending = None;
                    state.connected = false;
                    state.status = ScanStatus::Timeout;
                    state.timeouts += 1;
                    self.emit(scan, tx, peer, NetworkEventKind::Timeout);
                }
            }
        }
    }

    pub fn scan_count(&self) -> usize {
        self.scans.len()
    }
    pub fn scan_status(&self, scan: usize) -> Option<ScanStatus> {
        self.scans.get(scan).map(|s| s.status)
    }

    /// Decoded value for a mapping of a scan, from the last good response.
    pub fn value(&self, scan: usize, mapping: usize) -> Option<ScanValue> {
        let s = self.scans.get(scan)?;
        let m = s.config.values.get(mapping)?;
        let at = usize::from(m.offset);
        match m.data_type {
            ModbusDataType::Int16 => s.registers.get(at).map(|w| ScanValue::Int(*w as i16)),
            ModbusDataType::Float32 { word_order } => {
                let (a, b) = (*s.registers.get(at)?, *s.registers.get(at + 1)?);
                let (high, low) = match word_order {
                    WordOrder::HighWordFirst => (a, b),
                    WordOrder::LowWordFirst => (b, a),
                };
                let v = f32::from_bits((u32::from(high) << 16) | u32::from(low));
                Some(if v.is_finite() { ScanValue::Real(if v == 0.0 { 0.0 } else { v }) } else { ScanValue::NotFinite })
            }
        }
    }

    pub fn scan_views(&self) -> Vec<ScanView> {
        self.scans
            .iter()
            .map(|s| ScanView {
                id: s.config.id.clone(),
                server: s.config.server,
                unit_id: s.config.unit_id,
                status: s.status,
                connected: s.connected,
                requests: s.requests,
                responses: s.responses,
                timeouts: s.timeouts,
                exceptions: s.exceptions,
                last_round_trip_us: s.last_round_trip,
                registers: s.registers.clone(),
            })
            .collect()
    }

    pub fn device_views(&self) -> Vec<DeviceView> {
        self.devices
            .iter()
            .map(|d| DeviceView {
                id: d.def.id.clone(),
                label: d.def.label.clone(),
                ip: d.ip,
                mask: d.def.interface.mask,
                link_up: d.link_up,
                online: d.online,
                modbus_server: matches!(d.def.role, DeviceRole::ModbusTcpServer { .. }),
                requests_served: d.served,
                last_activity: d.last_activity.map(WorldTime::from_micros),
            })
            .collect()
    }

    pub fn plc_link_up(&self) -> bool {
        self.plc_link_up
    }
    pub fn plc_last_activity(&self) -> Option<WorldTime> {
        self.plc_activity.map(WorldTime::from_micros)
    }

    /// ICMP-style reachability check from the PLC, answered synchronously from topology.
    pub fn ping(&self, target: Ipv4) -> PingResult {
        let Some(interface) = &self.interface else { return PingResult::NoInterface };
        if !self.plc_link_up {
            return PingResult::Timeout;
        }
        if !interface.ip.same_subnet(target, interface.mask) && interface.gateway.is_none() {
            return PingResult::NoRoute;
        }
        let replies: Vec<&DeviceState> = self.devices.iter().filter(|d| d.ip == target && d.link_up && d.online).collect();
        match replies.as_slice() {
            [] => PingResult::Timeout,
            [d] if d.ip.same_subnet(interface.ip, d.def.interface.mask) || d.def.interface.gateway.is_some() => {
                PingResult::Reply { round_trip_us: 2 * self.latency }
            }
            [_] => PingResult::Timeout,
            _ => PingResult::DuplicateReplies,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PingResult {
    Reply { round_trip_us: u64 },
    Timeout,
    NoRoute,
    DuplicateReplies,
    NoInterface,
}

fn quantity(source: &dyn QuantitySource, component: &Identifier, name: &str) -> i64 {
    source.quantity(component.as_str(), name).unwrap_or(0)
}

/// Read one register word from a server table, or `None` if the address is not mapped.
fn register(table: &[lab_model::network::RegisterBinding], address: u16, source: &dyn QuantitySource) -> Option<u16> {
    for binding in table {
        match &binding.value {
            RegisterValue::Constant { value } if binding.address == address => return Some(*value),
            RegisterValue::ScaledInt16 { component, quantity: q, scale, divisor } if binding.address == address => {
                let v = quantity(source, component, q) * i64::from(*scale) / i64::from(*divisor);
                return Some(v.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16 as u16);
            }
            RegisterValue::Float32 { component, quantity: q, divisor, word_order } => {
                if address == binding.address || Some(address) == binding.address.checked_add(1) {
                    let v = quantity(source, component, q) as f64 / f64::from(*divisor);
                    let bits = (v as f32).to_bits();
                    let (high, low) = ((bits >> 16) as u16, bits as u16);
                    let (first, second) = match word_order {
                        WordOrder::HighWordFirst => (high, low),
                        WordOrder::LowWordFirst => (low, high),
                    };
                    return Some(if address == binding.address { first } else { second });
                }
            }
            _ => {}
        }
    }
    None
}
