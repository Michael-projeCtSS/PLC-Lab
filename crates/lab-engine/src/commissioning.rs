//! Commissioning checklist: the checks a commissioning engineer works through, evaluated
//! from what the installation shows (module status, field supplies, input readings and
//! their quality, outputs, device indications, network status, PLC state) and from the I/O
//! tests the student has performed in this session.
//!
//! Nothing here reads injected faults. Every finding is an observation the student could
//! also make with the other tools, compared with the drawing (the scene's documented
//! wiring and devices) where a commissioning engineer would compare it.

use crate::{plc::PlcMode, session::LabSession};
use hardware_model::{
    ChannelValue, DiagnosticKind, FindingCategory, FindingSeverity, HardwareFinding, LedState,
    Measurement, Quality,
};
use lab_model::{
    hardware::{ChannelRef, ModuleClass, SlotRef},
    scene::{AnalogSignal, ComponentKind, ContactType, PortKind, PortRole, TransmitterWiring},
};
use network_core::ScanStatus;
use process_simulation::Quantity;
use std::collections::{BTreeMap, BTreeSet};

/// How long an actuator must have been energized before missing feedback is a finding.
pub const FEEDBACK_WAIT_US: u64 = 3_000_000;
/// A terminal above this voltage counts as switched on.
const LIVE_MV: i64 = 12_000;
/// Findings listed per item before they are summarized.
const MAX_FINDINGS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckState {
    /// Nothing in this machine to check.
    NotApplicable,
    /// Checked and as documented.
    Ok,
    /// Needs a test the student has not performed yet.
    Untested,
    /// Deviates from the drawing or needs a closer look.
    Attention,
    /// Prevents correct operation.
    Fault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckItem {
    pub id: &'static str,
    pub title: &'static str,
    pub state: CheckState,
    /// Observed facts, most important first.
    pub findings: Vec<String>,
    /// What to do or which tool shows it.
    pub how: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckCategory {
    pub id: &'static str,
    pub title: &'static str,
    pub items: Vec<CheckItem>,
}

impl CheckCategory {
    /// The most severe state of the category's items.
    pub fn state(&self) -> CheckState {
        self.items
            .iter()
            .map(|i| i.state)
            .max()
            .unwrap_or(CheckState::NotApplicable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checklist {
    pub categories: Vec<CheckCategory>,
}

/// I/O tests observed in this session, updated once per cycle after the inputs were sampled.
#[derive(Debug, Clone, Default)]
pub(crate) struct IoTests {
    inputs: BTreeMap<usize, InputTest>,
    outputs: BTreeMap<ChannelRef, OutputTest>,
    time_us: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct InputTest {
    actuated: bool,
    edges: u32,
    /// Input channel -> readings seen [at rest, actuated][FALSE, TRUE].
    seen: BTreeMap<ChannelRef, [[bool; 2]; 2]>,
    /// The drawn channel's reading while the device was not actuated.
    rest: Option<bool>,
}

impl InputTest {
    /// Channels whose reading is determined by the device: one value at rest, the other
    /// while actuated. Consequences of the machine reacting (a contactor pulling in while
    /// Start is held) show both values for one device state and do not count.
    fn followed(&self, candidates: &BTreeSet<ChannelRef>) -> BTreeSet<ChannelRef> {
        self.seen
            .iter()
            .filter(|(c, _)| candidates.contains(c))
            .filter(|(_, [rest, actuated])| {
                rest.iter().filter(|v| **v).count() == 1
                    && actuated.iter().filter(|v| **v).count() == 1
                    && rest != actuated
            })
            .map(|(c, _)| *c)
            .collect()
    }
}

struct AnalogChecks {
    signal: CheckItem,
    raw: CheckItem,
    scaling: CheckItem,
}

#[derive(Debug, Clone, Default)]
struct OutputTest {
    commanded: bool,
    live: bool,
    received: bool,
    received_while_off: bool,
    energized_us: u64,
    /// Feedback channel -> values seen [while idle, while energized][FALSE, TRUE].
    feedback: BTreeMap<ChannelRef, [[bool; 2]; 2]>,
}

/// A device the student operates or the process actuates, and the quantity that shows it.
/// For a sensor port (port metadata), the quantity that shows the device actuated.
fn actuation(kind: &ComponentKind, port: &str) -> Option<&'static str> {
    if kind.port_role(port) != Some(PortRole::Sensor) {
        return None;
    }
    match kind {
        ComponentKind::PushButton { .. } => Some("pressed"),
        ComponentKind::PresenceSensor { .. } => Some("present"),
        ComponentKind::LevelSwitch { .. } => Some("wet"),
        ComponentKind::SelectorSwitch { .. } => Some("position"),
        _ => None,
    }
}

/// The selector position a selector port stands for: its index among the declared ports.
fn port_position(kind: &ComponentKind, port: &str) -> Option<i64> {
    kind.ports()
        .iter()
        .position(|(name, _)| *name == port)
        .and_then(|i| i64::try_from(i).ok())
}

/// The drawn contact type of an input device, when it has one.
fn contact_of(kind: &ComponentKind) -> Option<ContactType> {
    match kind {
        ComponentKind::PushButton { contact, .. }
        | ComponentKind::PresenceSensor { contact, .. }
        | ComponentKind::LevelSwitch { contact, .. } => Some(*contact),
        _ => None,
    }
}

/// Source ports that report an actuator's response to its command.
/// Ports that report an actuator's response to its command (port metadata).
fn feedback_ports(kind: &ComponentKind) -> Vec<&'static str> {
    kind.ports()
        .into_iter()
        .filter(|(name, _)| kind.port_role(name) == Some(PortRole::Feedback))
        .map(|(name, _)| name)
        .collect()
}

/// Whether a feedback input changed with its actuator: it showed a value while energized
/// that it did not show while idle (or both values while energized).
fn follows(seen: &[[bool; 2]; 2]) -> bool {
    let [idle, energized] = seen;
    energized[0] && energized[1]
        || (idle.iter().any(|v| *v) && energized.iter().any(|v| *v) && idle != energized)
}

fn is_on(q: Option<Quantity>) -> bool {
    matches!(q, Some(Quantity::Bool(true)))
}

fn slot_text(slot: SlotRef) -> String {
    format!("rack {} slot {}", slot.rack, slot.slot)
}

fn signal_text(s: AnalogSignal) -> &'static str {
    match s {
        AnalogSignal::Current4To20mA => "4-20 mA",
        AnalogSignal::Current0To20mA => "0-20 mA",
        AnalogSignal::Voltage0To10V => "0-10 V",
    }
}

fn bool_text(v: bool) -> &'static str {
    if v { "TRUE" } else { "FALSE" }
}

fn quality_text(q: Quality) -> &'static str {
    match q {
        Quality::Good => "good",
        Quality::NotMapped => "not mapped",
        Quality::ChannelDisabled => "channel disabled",
        Quality::ModuleFault => "module fault",
        Quality::NoFieldPower => "no field power",
        Quality::WireBreak => "wire break",
        Quality::Overflow => "overflow",
        Quality::Underflow => "underflow",
        Quality::Overrange => "over range",
        Quality::Underrange => "under range",
    }
}

fn led_text(led: LedState) -> &'static str {
    match led {
        LedState::Off => "off",
        LedState::Green => "green",
        LedState::GreenFlashing => "green flashing",
        LedState::Red => "red",
        LedState::RedFlashing => "red flashing",
        LedState::Amber => "amber",
    }
}

fn diagnostic_text(kind: DiagnosticKind) -> String {
    match kind {
        DiagnosticKind::ModuleMissing => "module missing".into(),
        DiagnosticKind::ModuleMismatch { installed, .. } => {
            format!("wrong module type ({} installed)", installed.generic_name())
        }
        DiagnosticKind::UnconfiguredModule => "module not in the project".into(),
        DiagnosticKind::NoFieldPower => "no field (load) supply".into(),
        DiagnosticKind::WireBreak => "wire break".into(),
        DiagnosticKind::Overflow => "overflow".into(),
        DiagnosticKind::Underflow => "underflow".into(),
        DiagnosticKind::ConnectionTimeout => "connection timeout".into(),
        DiagnosticKind::ConnectionRefused => "connection refused".into(),
        DiagnosticKind::NoRoute => "no route to the device".into(),
        DiagnosticKind::ModbusException(code) => format!("Modbus exception {code}"),
        DiagnosticKind::IpAddressConflict => "IP address conflict".into(),
        DiagnosticKind::LinkDown => "network link down".into(),
        DiagnosticKind::PlcFault => "PLC fault".into(),
        DiagnosticKind::ForceActive => "force active".into(),
        DiagnosticKind::ModeChange { .. } => "operating state changed".into(),
        DiagnosticKind::ProgramDownloaded => "program downloaded".into(),
        DiagnosticKind::HardwareDownloaded => "hardware configuration downloaded".into(),
    }
}

/// Keep at most [`MAX_FINDINGS`] findings and say how many more there were.
fn bounded(mut findings: Vec<String>) -> Vec<String> {
    if findings.len() > MAX_FINDINGS {
        let more = findings.len() - MAX_FINDINGS;
        findings.truncate(MAX_FINDINGS);
        findings.push(format!("... and {more} more"));
    }
    findings
}

fn item(
    id: &'static str,
    title: &'static str,
    state: CheckState,
    findings: Vec<String>,
    how: &'static str,
) -> CheckItem {
    CheckItem {
        id,
        title,
        state,
        findings: bounded(findings),
        how,
    }
}

/// The worst of several per-device results; `Ok` when there are none.
fn worst(states: impl IntoIterator<Item = CheckState>) -> CheckState {
    states.into_iter().max().unwrap_or(CheckState::Ok)
}

impl IoTests {
    pub(crate) fn observe(&mut self, session: &LabSession) {
        let now = session.now().as_micros();
        let dt = self.time_us.map_or(0, |t| now.saturating_sub(t));
        self.time_us = Some(now);
        let scene = &session.scene;
        let station = &session.station;
        let readings: BTreeMap<ChannelRef, bool> = digital_inputs(session)
            .into_iter()
            .filter_map(|c| match station.reading(c) {
                Some(r) => match r.value {
                    ChannelValue::Bool(b) => Some((c, b)),
                    _ => None,
                },
                None => None,
            })
            .collect();
        for wire in &session.scene_def.installation.wiring {
            let Ok(index) = scene.index_of_str(wire.component.as_str()) else {
                continue;
            };
            let Some(definition) = session.scene_def.components.get(index) else {
                continue;
            };
            if let Some(quantity) = actuation(&definition.kind, &wire.port) {
                let actuated = match scene.quantity(index, quantity) {
                    Some(Quantity::Integer(p)) => {
                        port_position(&definition.kind, &wire.port) == Some(p)
                    }
                    other => is_on(other),
                };
                let test = self.inputs.entry(index).or_default();
                if actuated != test.actuated {
                    test.edges += 1;
                }
                test.actuated = actuated;
                for (channel, value) in &readings {
                    test.seen.entry(*channel).or_default()[usize::from(actuated)]
                        [usize::from(*value)] = true;
                }
                if !actuated && let Some(value) = readings.get(&wire.channel) {
                    test.rest = Some(*value);
                }
            }
            let sink = definition
                .kind
                .ports()
                .into_iter()
                .any(|(p, k)| p == wire.port && k == PortKind::DigitalSink);
            if sink {
                let commanded = station.output(wire.channel) == Some(true);
                let live = matches!(
                    station.measure(wire.channel, scene),
                    Measurement::Millivolts(mv) if mv > LIVE_MV
                );
                let received = definition
                    .kind
                    .command_quantity(&wire.port)
                    .is_some_and(|q| is_on(scene.quantity(index, q)));
                let test = self.outputs.entry(wire.channel).or_default();
                test.commanded |= commanded;
                test.live |= live;
                test.received |= received;
                test.received_while_off |= received && !commanded;
                if received {
                    test.energized_us += dt;
                }
                for feedback in &session.scene_def.installation.wiring {
                    if feedback.component == wire.component
                        && feedback_ports(&definition.kind).contains(&feedback.port.as_str())
                        && let Some(value) = readings.get(&feedback.channel)
                    {
                        test.feedback.entry(feedback.channel).or_default()[usize::from(received)]
                            [usize::from(*value)] = true;
                    }
                }
            }
        }
    }
}

/// Every digital input channel of the configured modules.
fn digital_inputs(session: &LabSession) -> Vec<ChannelRef> {
    session
        .station
        .configuration()
        .modules
        .iter()
        .filter(|m| matches!(m.module.class(), ModuleClass::DigitalInput { .. }))
        .flat_map(|m| {
            (0..m.module.channel_count()).map(move |channel| ChannelRef {
                rack: m.position.rack,
                slot: m.position.slot,
                channel,
            })
        })
        .collect()
}

impl LabSession {
    /// The commissioning checklist for the current state of the installation.
    pub fn commissioning(&self) -> Checklist {
        Checklist {
            categories: vec![
                self.power(),
                self.hardware(),
                self.network_checks(),
                self.inputs(),
                self.outputs(),
                self.drives(),
                self.process(),
            ],
        }
    }

    /// "%I0.0 (StartPB)" style channel name.
    fn channel_name(&self, channel: ChannelRef) -> String {
        let address = self.channel_address(channel);
        match self.station.tag_for(channel) {
            Some(tag) => format!("{address} ({tag})"),
            None => address,
        }
    }

    fn drives_present(&self) -> Vec<usize> {
        self.scene_def
            .components
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.kind, ComponentKind::VariableFrequencyDrive { .. }))
            .map(|(i, _)| i)
            .collect()
    }

    /// Field supply findings and the number of I/O modules concerned.
    fn io_modules(&self, outputs_only: bool) -> (Vec<String>, usize) {
        let relevant = |class: ModuleClass| match class {
            ModuleClass::DigitalOutput { .. } | ModuleClass::AnalogOutput { .. } => true,
            ModuleClass::DigitalInput { .. } | ModuleClass::AnalogInput { .. } => !outputs_only,
            _ => false,
        };
        let modules = self
            .station
            .configuration()
            .modules
            .iter()
            .filter(|m| relevant(m.module.class()))
            .count();
        let problems = self
            .station
            .compare(&self.scene)
            .into_iter()
            .filter(|f| f.category == FindingCategory::FieldPowerMissing)
            .filter(|f| {
                f.module_id
                    .as_deref()
                    .and_then(|id| self.station.configuration().module_by_id(id))
                    .is_some_and(|m| relevant(m.module.class()))
            })
            .map(|f| f.message)
            .collect();
        (problems, modules)
    }

    fn power(&self) -> CheckCategory {
        let cpu = self.scene_def.installation.cpu.generic_name();
        let (supply, modules) = self.io_modules(false);
        let electronics: Vec<String> = self
            .station
            .compare(&self.scene)
            .into_iter()
            .filter(|f| f.category == FindingCategory::ModulePowerMissing)
            .map(|f| f.message)
            .collect();
        let drives = self.drives_present();
        let drive_supply = if drives.is_empty() {
            item(
                "drive-supply",
                "Drive supply",
                CheckState::NotApplicable,
                vec!["This machine has no drive.".into()],
                "",
            )
        } else {
            let mut findings = Vec::new();
            let mut states = Vec::new();
            for &index in &drives {
                let supplied = is_on(self.scene.quantity(index, "supplied"));
                let label = &self.scene_def.components[index].label;
                states.push(if supplied {
                    CheckState::Ok
                } else {
                    CheckState::Fault
                });
                findings.push(if supplied {
                    format!("{label}: mains supply on")
                } else {
                    format!("{label}: no mains supply (display dark, not ready)")
                });
            }
            item(
                "drive-supply",
                "Drive supply",
                worst(states),
                findings,
                "Drive display; measure the mains supply only with qualified, safe equipment.",
            )
        };
        let devices = self.network.device_views();
        let comm = if devices.is_empty() {
            item(
                "comm-power",
                "Communication equipment",
                CheckState::NotApplicable,
                vec!["This machine has no network devices.".into()],
                "",
            )
        } else {
            let off: Vec<String> = devices
                .iter()
                .filter(|d| !d.online)
                .map(|d| format!("{} ({}) does not respond", d.label, d.ip))
                .collect();
            item(
                "comm-power",
                "Communication equipment",
                if off.is_empty() {
                    CheckState::Ok
                } else {
                    CheckState::Fault
                },
                if off.is_empty() {
                    vec![format!(
                        "{} network device(s) powered and online",
                        devices.len()
                    )]
                } else {
                    off
                },
                "Network view: device status; ping the device.",
            )
        };
        CheckCategory {
            id: "power",
            title: "Power",
            items: vec![
                item(
                    "controller",
                    "Controller powered",
                    CheckState::Ok,
                    vec![format!("{cpu} powered")],
                    "Hardware view: CPU status LEDs.",
                ),
                item(
                    "module-power",
                    "Module electronics supply",
                    if electronics.is_empty() {
                        CheckState::Ok
                    } else {
                        CheckState::Fault
                    },
                    if electronics.is_empty() {
                        vec!["Every configured module answers the CPU".into()]
                    } else {
                        electronics
                    },
                    "Hardware view: module status LEDs; a module without electronics supply does not answer.",
                ),
                item(
                    "io-supply",
                    "I/O field supply",
                    if supply.is_empty() {
                        CheckState::Ok
                    } else {
                        CheckState::Fault
                    },
                    if supply.is_empty() {
                        vec![format!("{modules} I/O module(s) have their field supply")]
                    } else {
                        supply
                    },
                    "Hardware view: module status; measure the module's 24 V supply.",
                ),
                drive_supply,
                comm,
            ],
        }
    }

    fn hardware(&self) -> CheckCategory {
        let configured = self.station.configuration().cpu;
        let installed = self.scene_def.installation.cpu;
        let views = self.station.module_views();
        let findings = self.station.compare(&self.scene);
        let of = |categories: &[FindingCategory]| -> Vec<&HardwareFinding> {
            findings
                .iter()
                .filter(|f| categories.contains(&f.category))
                .collect()
        };
        let texts = |list: &[&HardwareFinding]| -> Vec<String> {
            list.iter().map(|f| f.message.clone()).collect()
        };
        let types = texts(&of(&[FindingCategory::WrongModuleType]));
        let slot_findings = of(&[
            FindingCategory::ModuleMissing,
            FindingCategory::WrongSlot,
            FindingCategory::ModulePowerMissing,
            FindingCategory::UnexpectedModule,
        ]);
        let slots_fault = slot_findings
            .iter()
            .any(|f| f.severity == FindingSeverity::Error);
        let slots = texts(&slot_findings);
        let consistency = texts(&of(&[
            FindingCategory::AddressOverlap,
            FindingCategory::SignalBindingMismatch,
        ]));
        let mut leds = Vec::new();
        for view in &views {
            let name = view
                .configured
                .or(view.installed)
                .map_or("module", |m| m.generic_name());
            if view.status_led != LedState::Green {
                leds.push(format!(
                    "{} {name}: status LED {}",
                    slot_text(view.position),
                    led_text(view.status_led)
                ));
            }
        }
        let diagnostics: Vec<String> = self
            .station
            .active_diagnostics()
            .map(|(source, kind)| {
                let at = match source {
                    hardware_model::DiagnosticSource::Cpu => "CPU".to_owned(),
                    hardware_model::DiagnosticSource::Module(s) => slot_text(*s),
                    hardware_model::DiagnosticSource::Channel(c) => self.channel_name(*c),
                    hardware_model::DiagnosticSource::Connection(i) => format!("connection {i}"),
                    hardware_model::DiagnosticSource::Interface => "PLC interface".to_owned(),
                };
                format!("{at}: {}", diagnostic_text(*kind))
            })
            .collect();
        let plc = match self.plc_mode() {
            PlcMode::Run => item(
                "plc-state",
                "PLC state",
                CheckState::Ok,
                vec![match self.program() {
                    Some((name, revision, _)) => {
                        format!("RUN, program {name} (download {revision})")
                    }
                    None => "RUN".into(),
                }],
                "Toolbar: PLC mode and program.",
            ),
            PlcMode::Stop => item(
                "plc-state",
                "PLC state",
                CheckState::Attention,
                vec![if self.program().is_some() {
                    "STOP: the program is loaded but not executed".into()
                } else {
                    "STOP: no program loaded".into()
                }],
                "Toolbar: switch the PLC to RUN after Build & Load.",
            ),
            PlcMode::Fault => item(
                "plc-state",
                "PLC state",
                CheckState::Fault,
                vec![match self.plc_fault() {
                    Some(d) => format!("FAULT: {}", d.message()),
                    None => "FAULT".into(),
                }],
                "Diagnostics view: the PLC fault; reset the PLC after correcting it.",
            ),
        };
        let state = |problems: &Vec<String>, bad| {
            if problems.is_empty() {
                CheckState::Ok
            } else {
                bad
            }
        };
        CheckCategory {
            id: "hardware",
            title: "Hardware",
            items: vec![
                item(
                    "cpu",
                    "CPU matches the configuration",
                    if configured == installed {
                        CheckState::Ok
                    } else {
                        CheckState::Fault
                    },
                    vec![if configured == installed {
                        format!("{} installed and configured", installed.generic_name())
                    } else {
                        format!(
                            "project {}, installed {}",
                            configured.generic_name(),
                            installed.generic_name()
                        )
                    }],
                    "Hardware view: compare the project with the installed CPU.",
                ),
                item(
                    "module-types",
                    "Module types",
                    state(&types, CheckState::Fault),
                    if types.is_empty() {
                        vec!["Every installed module has the configured type".into()]
                    } else {
                        types.clone()
                    },
                    "Hardware view (Compare): configured against detected modules.",
                ),
                item(
                    "slot-order",
                    "Modules present in their slots",
                    if slots.is_empty() {
                        CheckState::Ok
                    } else if slots_fault {
                        CheckState::Fault
                    } else {
                        CheckState::Attention
                    },
                    if slots.is_empty() {
                        vec![format!("{} module position(s) as configured", views.len())]
                    } else {
                        slots
                    },
                    "Hardware view (Compare): configured against detected modules.",
                ),
                item(
                    "configuration",
                    "Addresses and I/O mapping",
                    state(&consistency, CheckState::Fault),
                    if consistency.is_empty() {
                        vec![
                            "No overlapping addresses; every mapping points to a usable channel"
                                .into(),
                        ]
                    } else {
                        consistency.clone()
                    },
                    "Hardware view: addresses and the signal mapping.",
                ),
                item(
                    "module-status",
                    "Module status",
                    state(&leds, CheckState::Attention),
                    if leds.is_empty() {
                        vec!["All module status LEDs green".into()]
                    } else {
                        leds.clone()
                    },
                    "Hardware view: module status LEDs.",
                ),
                item(
                    "diagnostics",
                    "Diagnostics",
                    state(&diagnostics, CheckState::Attention),
                    if diagnostics.is_empty() {
                        vec!["No active diagnostic".into()]
                    } else {
                        diagnostics.clone()
                    },
                    "Diagnostics view: the diagnostic buffer.",
                ),
                plc,
            ],
        }
    }

    fn network_checks(&self) -> CheckCategory {
        let interface = self.station.configuration().network.interface.clone();
        let devices = self.network.device_views();
        let scans = self.network.scan_views();
        let none = |id, title| {
            item(
                id,
                title,
                CheckState::NotApplicable,
                vec!["This machine has no network.".into()],
                "",
            )
        };
        if interface.is_none() && devices.is_empty() && scans.is_empty() {
            return CheckCategory {
                id: "network",
                title: "Network",
                items: vec![
                    none("address", "Address"),
                    none("link", "Link"),
                    none("devices", "Station and device names"),
                    none("quality", "Communication quality"),
                    none("cyclic", "Cyclic communication"),
                ],
            };
        }
        let conflict = self
            .station
            .active_diagnostics()
            .any(|(_, k)| *k == DiagnosticKind::IpAddressConflict);
        let address = match &interface {
            Some(i) if !conflict => item(
                "address",
                "Address",
                CheckState::Ok,
                vec![format!("PLC interface {} / {}", i.ip, i.mask)],
                "Network configuration: the PLC interface.",
            ),
            Some(i) => item(
                "address",
                "Address",
                CheckState::Fault,
                vec![format!("{} is used by another device", i.ip)],
                "Network view: device addresses.",
            ),
            None => item(
                "address",
                "Address",
                CheckState::Fault,
                vec!["The PLC interface has no IP address".into()],
                "Network configuration: set the PLC interface address.",
            ),
        };
        let mut down: Vec<String> = Vec::new();
        if !self.network.plc_link_up() {
            down.push("PLC port: no link".into());
        }
        down.extend(
            devices
                .iter()
                .filter(|d| !d.link_up)
                .map(|d| format!("{}: no link", d.label)),
        );
        let link = item(
            "link",
            "Link",
            if down.is_empty() {
                CheckState::Ok
            } else {
                CheckState::Fault
            },
            if down.is_empty() {
                vec!["All ports have link".into()]
            } else {
                down
            },
            "Network view: link LEDs; check the cable.",
        );
        let names = item(
            "devices",
            "Station and device names",
            if devices.iter().all(|d| d.online) {
                CheckState::Ok
            } else {
                CheckState::Fault
            },
            devices
                .iter()
                .map(|d| {
                    format!(
                        "{} at {}: {}",
                        d.label,
                        d.ip,
                        if d.online { "online" } else { "not responding" }
                    )
                })
                .collect(),
            "Network view: devices; ping a device by its address.",
        );
        let errors: Vec<String> = scans
            .iter()
            .filter(|s| s.timeouts > 0 || s.exceptions > 0)
            .map(|s| {
                format!(
                    "{} ({}): {} timeout(s), {} exception(s) in {} request(s)",
                    s.id, s.server, s.timeouts, s.exceptions, s.requests
                )
            })
            .collect();
        let quality = if scans.is_empty() {
            none("quality", "Communication quality")
        } else {
            item(
                "quality",
                "Communication quality",
                if errors.is_empty() {
                    CheckState::Ok
                } else {
                    CheckState::Attention
                },
                if errors.is_empty() {
                    vec!["No timeouts or exceptions".into()]
                } else {
                    errors
                },
                "Network view: request and error counters.",
            )
        };
        let cyclic = if scans.is_empty() {
            none("cyclic", "Cyclic communication")
        } else {
            let states: Vec<CheckState> = scans
                .iter()
                .map(|s| match s.status {
                    ScanStatus::Ok => CheckState::Ok,
                    ScanStatus::Idle => CheckState::Untested,
                    _ => CheckState::Fault,
                })
                .collect();
            item(
                "cyclic",
                "Cyclic communication",
                worst(states),
                scans
                    .iter()
                    .map(|s| {
                        let status = match s.status {
                            ScanStatus::Idle => "idle (the PLC is not polling)".to_owned(),
                            ScanStatus::Ok => {
                                format!("exchanging data ({} responses)", s.responses)
                            }
                            ScanStatus::Timeout => "timeout".to_owned(),
                            ScanStatus::NoRoute => "no route".to_owned(),
                            ScanStatus::ConnectionRefused => "connection refused".to_owned(),
                            ScanStatus::Exception(code) => format!("exception {code}"),
                            ScanStatus::NoInterface => "no PLC interface".to_owned(),
                        };
                        format!("{} -> {}: {status}", s.id, s.server)
                    })
                    .collect(),
                "Network view: connection status; the PLC polls only in RUN.",
            )
        };
        CheckCategory {
            id: "network",
            title: "Network",
            items: vec![address, link, names, quality, cyclic],
        }
    }

    fn inputs(&self) -> CheckCategory {
        // Where an operator device or sensor wire can land: inputs drawn for such devices
        // and spare inputs. Feedback contacts of actuators react to the machine, not to
        // the device being tested.
        let feedback: BTreeSet<ChannelRef> = self
            .scene_def
            .installation
            .wiring
            .iter()
            .filter(|w| {
                self.scene
                    .index_of_str(w.component.as_str())
                    .is_ok_and(|i| actuation(&self.scene_def.components[i].kind, &w.port).is_none())
            })
            .map(|w| w.channel)
            .collect();
        let candidates: BTreeSet<ChannelRef> = digital_inputs(self)
            .into_iter()
            .filter(|c| !feedback.contains(c))
            .collect();
        let mut bad = Vec::new();
        let mut values = Vec::new();
        for channel in digital_inputs(self) {
            let Some(reading) = self.station.reading(channel) else {
                continue;
            };
            let mapped = self.station.tag_for(channel).is_some();
            if reading.quality != Quality::Good && reading.quality != Quality::NotMapped {
                bad.push(format!(
                    "{}: {}",
                    self.channel_name(channel),
                    quality_text(reading.quality)
                ));
            } else if mapped && let ChannelValue::Bool(v) = reading.value {
                values.push(format!("{} = {}", self.channel_name(channel), bool_text(v)));
            }
        }
        let tests = &self.tests;
        let mut operation = Vec::new();
        let mut operation_states = Vec::new();
        let mut contacts = Vec::new();
        let mut contact_states = Vec::new();
        for wire in &self.scene_def.installation.wiring {
            let Ok(index) = self.scene.index_of_str(wire.component.as_str()) else {
                continue;
            };
            let definition = &self.scene_def.components[index];
            if actuation(&definition.kind, &wire.port).is_none() {
                continue;
            }
            let label = &definition.label;
            let drawn = self.channel_name(wire.channel);
            let test = tests.inputs.get(&index);
            let (state, finding) = match test {
                None | Some(InputTest { edges: 0, .. }) => (
                    CheckState::Untested,
                    format!("{label}: not operated yet (drawn on {drawn})"),
                ),
                Some(t) if t.followed(&candidates).contains(&wire.channel) => (
                    CheckState::Ok,
                    format!("{label}: {drawn} follows the device"),
                ),
                Some(t) if !t.followed(&candidates).is_empty() => (
                    CheckState::Fault,
                    format!(
                        "{label}: operating it changed {} instead of {drawn}",
                        t.followed(&candidates)
                            .iter()
                            .map(|c| self.channel_name(*c))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ),
                Some(_) => (
                    CheckState::Fault,
                    format!("{label}: operating it changed no input; {drawn} did not follow"),
                ),
            };
            operation_states.push(state);
            operation.push(finding);
            if let Some(contact) = contact_of(&definition.kind) {
                let expected = contact == ContactType::NormallyClosed;
                let drawing = if expected {
                    "normally closed (TRUE at rest)"
                } else {
                    "normally open (FALSE at rest)"
                };
                let (state, finding) = match test {
                    Some(t) if t.edges > 0 && t.rest.is_some() => {
                        let rest = t.rest == Some(true);
                        if rest == expected {
                            (
                                CheckState::Ok,
                                format!(
                                    "{label}: drawn {drawing}, reads {} at rest",
                                    bool_text(rest)
                                ),
                            )
                        } else {
                            (
                                CheckState::Attention,
                                format!(
                                    "{label}: drawn {drawing}, but {drawn} reads {} at rest",
                                    bool_text(rest)
                                ),
                            )
                        }
                    }
                    _ => (
                        CheckState::Untested,
                        format!("{label}: drawn {drawing}; operate it to compare"),
                    ),
                };
                contact_states.push(state);
                contacts.push(finding);
            }
        }
        let analog = self.analog_checks();
        let no_devices = |findings: Vec<String>| {
            if findings.is_empty() {
                vec!["No operator devices or sensors are wired to inputs.".into()]
            } else {
                findings
            }
        };
        CheckCategory {
            id: "inputs",
            title: "Inputs",
            items: vec![
                item(
                    "di-state",
                    "Digital input state",
                    if bad.is_empty() {
                        CheckState::Ok
                    } else {
                        CheckState::Fault
                    },
                    bad.into_iter().chain(values).collect(),
                    "Hardware view: input LEDs; Watch: input values and quality.",
                ),
                item(
                    "sensor-operation",
                    "Sensor and button operation",
                    if operation_states.is_empty() {
                        CheckState::NotApplicable
                    } else {
                        worst(operation_states)
                    },
                    no_devices(operation),
                    "Operate each device in the Process view and watch its input follow.",
                ),
                item(
                    "contact-type",
                    "NO/NC as drawn",
                    if contact_states.is_empty() {
                        CheckState::NotApplicable
                    } else {
                        worst(contact_states)
                    },
                    no_devices(contacts),
                    "Compare the input at rest with the drawing: NC reads TRUE at rest, NO FALSE.",
                ),
                analog.signal,
                analog.raw,
                analog.scaling,
            ],
        }
    }

    /// Analog inputs wired to transmitters: signal type and loop against the drawing, the
    /// raw value and its condition, and the transmitter value against the sight glass.
    fn analog_checks(&self) -> AnalogChecks {
        let findings = self.station.compare(&self.scene);
        let mut signal = (Vec::new(), Vec::new());
        let mut raw = (Vec::new(), Vec::new());
        let mut scaling = (Vec::new(), Vec::new());
        for wire in &self.scene_def.installation.wiring {
            let Ok(index) = self.scene.index_of_str(wire.component.as_str()) else {
                continue;
            };
            let definition = &self.scene_def.components[index];
            let ComponentKind::LevelTransmitter {
                tank,
                range_min_ml,
                range_max_ml,
                signal: drawn,
                wiring,
            } = &definition.kind
            else {
                continue;
            };
            let label = &definition.label;
            let name = self.channel_name(wire.channel);
            let current = matches!(
                drawn,
                AnalogSignal::Current4To20mA | AnalogSignal::Current0To20mA
            );
            // Signal type and loop: the station comparison against the drawing.
            let two_wire = *wiring == TransmitterWiring::TwoWire;
            let mismatch = findings.iter().find(|f| {
                f.channel == Some(wire.channel)
                    && matches!(
                        f.category,
                        FindingCategory::ChannelModeMismatch | FindingCategory::LoopSupplyMismatch
                    )
            });
            let (state, finding) = if let Some(f) = mismatch {
                (CheckState::Fault, f.message.clone())
            } else {
                let power = match (current, two_wire) {
                    (true, true) => "2-wire, loop supplied by the input",
                    (true, false) => "4-wire, passive input",
                    (false, _) => "separately powered",
                };
                (
                    CheckState::Ok,
                    format!("{label}: {} {power}; {name} matches", signal_text(*drawn)),
                )
            };
            signal.0.push(state);
            signal.1.push(finding);
            // Raw value and its condition.
            let measured = match self.station.measure(wire.channel, &self.scene) {
                Measurement::Microamps(ua) => Some((ua, "mA")),
                Measurement::Millivolts(mv) => Some((mv, "V")),
                Measurement::NoModule => None,
            };
            let (state, finding) = match (self.station.reading(wire.channel), measured) {
                (Some(r), Some((value, unit))) => {
                    let raw_text = match r.value {
                        ChannelValue::Int(i) => i.to_string(),
                        ChannelValue::Real(v) => format!("{v}"),
                        ChannelValue::Bool(b) => bool_text(b).to_owned(),
                    };
                    let state = match r.quality {
                        Quality::Good => CheckState::Ok,
                        Quality::Overrange | Quality::Underrange => CheckState::Attention,
                        _ => CheckState::Fault,
                    };
                    (
                        state,
                        format!(
                            "{label} on {name}: {}.{:02} {unit}, raw {raw_text} ({})",
                            value / 1000,
                            (value % 1000).abs() / 10,
                            quality_text(r.quality)
                        ),
                    )
                }
                _ => (CheckState::Fault, format!("{label} on {name}: no reading")),
            };
            let valid = state != CheckState::Fault;
            raw.0.push(state);
            raw.1.push(finding);
            // The value the signal stands for (drawn range) against the tank's sight glass.
            let span = i64::try_from(range_max_ml.saturating_sub(*range_min_ml).max(1))
                .unwrap_or(i64::MAX);
            let min = i64::try_from(*range_min_ml).unwrap_or(0);
            let ppm = measured.map(|(value, _)| match drawn {
                AnalogSignal::Current4To20mA => (value - 4_000) * 1_000_000 / 16_000,
                AnalogSignal::Current0To20mA => value * 1_000_000 / 20_000,
                AnalogSignal::Voltage0To10V => value * 1_000_000 / 10_000,
            });
            let level = self.scene.index_of_str(tank.as_str()).ok().and_then(|t| {
                match self.scene.quantity(t, "levelMl") {
                    Some(Quantity::Integer(ml)) => Some(ml),
                    _ => None,
                }
            });
            let (state, finding) = match (ppm, level) {
                (Some(ppm), Some(level)) if valid => {
                    let value = min + ppm * span / 1_000_000;
                    let deviation = (value - level).abs();
                    let text = format!(
                        "{label}: {}.{} % of {range_min_ml}..{range_max_ml} ml = {value} ml; sight glass {level} ml",
                        ppm / 10_000,
                        (ppm % 10_000).abs() / 1_000
                    );
                    // Within 2 % of the span counts as matching.
                    if deviation * 50 <= span {
                        (CheckState::Ok, text)
                    } else {
                        (
                            CheckState::Attention,
                            format!("{text}: differs by {deviation} ml"),
                        )
                    }
                }
                _ => (
                    CheckState::Untested,
                    format!("{label}: no valid signal to compare with the sight glass"),
                ),
            };
            scaling.0.push(state);
            scaling.1.push(finding);
        }
        let make = |id, title, (states, findings): (Vec<CheckState>, Vec<String>), how| {
            if states.is_empty() {
                item(
                    id,
                    title,
                    CheckState::NotApplicable,
                    vec!["This machine has no analog transmitters.".into()],
                    "",
                )
            } else {
                item(id, title, worst(states), findings, how)
            }
        };
        AnalogChecks {
            signal: make(
                "analog-signal",
                "Analog signal type and loop",
                signal,
                "Hardware view (Advanced): channel mode and loop supply against the drawing.",
            ),
            raw: make(
                "analog-raw",
                "Analog raw value",
                raw,
                "Measure the loop current or voltage at the terminal; Watch: the raw value.",
            ),
            scaling: make(
                "scaling",
                "Scaling",
                scaling,
                "Compare the transmitter value with the sight glass, then with your scaled value in Watch.",
            ),
        }
    }

    fn outputs(&self) -> CheckCategory {
        let mut logical = Vec::new();
        let mut logical_states = Vec::new();
        let mut physical = Vec::new();
        let mut physical_states = Vec::new();
        let mut response = Vec::new();
        let mut response_states = Vec::new();
        let mut feedback = Vec::new();
        let mut feedback_states = Vec::new();
        for wire in &self.scene_def.installation.wiring {
            let Ok(index) = self.scene.index_of_str(wire.component.as_str()) else {
                continue;
            };
            let definition = &self.scene_def.components[index];
            let sink = definition
                .kind
                .ports()
                .into_iter()
                .any(|(p, k)| p == wire.port && k == PortKind::DigitalSink);
            if !sink {
                continue;
            }
            let label = &definition.label;
            let name = self.channel_name(wire.channel);
            let now = self.station.output(wire.channel) == Some(true);
            let test = self
                .tests
                .outputs
                .get(&wire.channel)
                .cloned()
                .unwrap_or_default();
            logical.push(format!(
                "{name} -> {label}: {}",
                if now { "ON" } else { "OFF" }
            ));
            logical_states.push(if test.commanded {
                CheckState::Ok
            } else {
                CheckState::Untested
            });
            let (state, finding) = if !test.commanded {
                (CheckState::Untested, format!("{name}: not switched ON yet"))
            } else if test.live {
                (CheckState::Ok, format!("{name}: terminal switches 24 V"))
            } else {
                (
                    CheckState::Fault,
                    format!("{name}: commanded ON, the terminal stayed at 0 V"),
                )
            };
            physical_states.push(state);
            physical.push(finding);
            let (state, finding) = if test.received_while_off {
                (
                    CheckState::Fault,
                    format!("{label}: energized although {name} was OFF"),
                )
            } else if !test.live {
                (CheckState::Untested, format!("{label}: not energized yet"))
            } else if test.received {
                (CheckState::Ok, format!("{label}: responds to {name}"))
            } else {
                (
                    CheckState::Fault,
                    format!("{name} switched 24 V, but {label} did not respond"),
                )
            };
            response_states.push(state);
            response.push(finding);
            for (channel, seen) in &test.feedback {
                let feedback_name = self.channel_name(*channel);
                let (state, finding) = if follows(seen) {
                    (
                        CheckState::Ok,
                        format!("{label}: feedback {feedback_name} follows"),
                    )
                } else if test.energized_us >= FEEDBACK_WAIT_US {
                    (
                        CheckState::Fault,
                        format!(
                            "{label}: energized for {} s, feedback {feedback_name} never changed",
                            test.energized_us / 1_000_000
                        ),
                    )
                } else {
                    (
                        CheckState::Untested,
                        format!("{label}: energize it for a few seconds to see {feedback_name}"),
                    )
                };
                feedback_states.push(state);
                feedback.push(finding);
            }
            if test.feedback.is_empty() && !feedback_ports(&definition.kind).is_empty() {
                feedback_states.push(CheckState::Untested);
                feedback.push(format!("{label}: energize it to see its feedback"));
            }
        }
        let (supply, modules) = self.io_modules(true);
        let applicable = |states: Vec<CheckState>| {
            if states.is_empty() {
                CheckState::NotApplicable
            } else {
                worst(states)
            }
        };
        let none = |findings: Vec<String>, what: &str| {
            if findings.is_empty() {
                vec![format!("No {what}.")]
            } else {
                findings
            }
        };
        CheckCategory {
            id: "outputs",
            title: "Outputs",
            items: vec![
                item(
                    "logical",
                    "Logical PLC output",
                    applicable(logical_states),
                    none(logical, "wired outputs"),
                    "Switch each output ON from the program or with a force.",
                ),
                item(
                    "physical",
                    "Physical output",
                    applicable(physical_states),
                    none(physical, "wired outputs"),
                    "Hardware view: output LED; measure the terminal against M.",
                ),
                item(
                    "field-supply",
                    "Output field supply",
                    if modules == 0 {
                        CheckState::NotApplicable
                    } else if supply.is_empty() {
                        CheckState::Ok
                    } else {
                        CheckState::Fault
                    },
                    if modules == 0 {
                        vec!["No output modules.".into()]
                    } else if supply.is_empty() {
                        vec![format!("{modules} output module(s) have their load supply")]
                    } else {
                        supply
                    },
                    "Hardware view: output module status.",
                ),
                item(
                    "actuator",
                    "Actuator response",
                    applicable(response_states),
                    none(response, "wired actuators"),
                    "Process view: does the device move when its output switches?",
                ),
                item(
                    "feedback",
                    "Feedback",
                    applicable(feedback_states),
                    none(feedback, "actuators with feedback"),
                    "Watch: the feedback input while the actuator is energized.",
                ),
            ],
        }
    }

    fn drives(&self) -> CheckCategory {
        const ITEMS: [(&str, &str, &str); 7] = [
            (
                "ready",
                "Ready",
                "Drive display and ready relay: supply, fault and enable.",
            ),
            (
                "control-place",
                "Local / remote",
                "Drive display: control place.",
            ),
            (
                "enable",
                "Run enable",
                "Watch the enable output and the drive's enable terminal.",
            ),
            (
                "fault",
                "Fault",
                "Drive display and fault relay; acknowledge after removing the cause.",
            ),
            (
                "run-command",
                "Run command",
                "Command the drive from the PLC and watch its run input.",
            ),
            (
                "reference",
                "Speed reference",
                "Drive display: reference source and value.",
            ),
            (
                "speed",
                "Speed and feedback",
                "Process view: drive speed; Watch: the running feedback.",
            ),
        ];
        let drives = self.drives_present();
        if drives.is_empty() {
            return CheckCategory {
                id: "drives",
                title: "Drives",
                items: ITEMS
                    .iter()
                    .map(|(id, title, _)| {
                        item(
                            id,
                            title,
                            CheckState::NotApplicable,
                            vec!["This machine has no drive.".into()],
                            "",
                        )
                    })
                    .collect(),
            };
        }
        let mut results: Vec<(Vec<CheckState>, Vec<String>)> = vec![Default::default(); 7];
        for index in drives {
            let definition = &self.scene_def.components[index];
            let label = &definition.label;
            let enable_terminal = matches!(
                definition.kind,
                ComponentKind::VariableFrequencyDrive {
                    enable_terminal: true,
                    ..
                }
            );
            let q = |name| is_on(self.scene.quantity(index, name));
            let permille = |name| match self.scene.quantity(index, name) {
                Some(Quantity::Integer(p)) => p,
                _ => 0,
            };
            let percent = |p: i64| format!("{}.{} %", p / 10, p % 10);
            let (ready, tripped, commanded) = (q("ready"), q("tripped"), q("command"));
            // Control place, fault and reference are read on the drive's display, which is
            // dark without supply.
            let supplied = q("supplied");
            let mut add = |item: usize, state: CheckState, finding: String| {
                let (state, finding) = if !supplied && matches!(item, 1 | 3 | 5) {
                    (
                        CheckState::Untested,
                        format!("{label}: no indication, the drive has no supply"),
                    )
                } else {
                    (state, finding)
                };
                results[item].0.push(state);
                results[item].1.push(finding);
            };
            let mut reasons = Vec::new();
            if !supplied {
                reasons.push("no mains supply");
            }
            if tripped {
                reasons.push("faulted");
            }
            if enable_terminal && !q("enableCommand") {
                reasons.push("run enable missing");
            }
            add(
                0,
                if ready {
                    CheckState::Ok
                } else if commanded {
                    CheckState::Fault
                } else {
                    CheckState::Attention
                },
                if ready {
                    format!("{label}: ready")
                } else {
                    format!(
                        "{label}: not ready ({}){}",
                        reasons.join(", "),
                        if commanded {
                            "; the run command is active"
                        } else {
                            ""
                        }
                    )
                },
            );
            let remote = q("remote");
            add(
                1,
                if remote {
                    CheckState::Ok
                } else {
                    CheckState::Attention
                },
                if remote {
                    format!("{label}: remote (follows the PLC)")
                } else {
                    format!("{label}: local control, the PLC run command is ignored")
                },
            );
            add(
                2,
                match (enable_terminal, q("enableCommand")) {
                    (false, _) => CheckState::NotApplicable,
                    (true, true) => CheckState::Ok,
                    (true, false) => CheckState::Attention,
                },
                if enable_terminal {
                    format!(
                        "{label}: run enable terminal {}",
                        if q("enableCommand") {
                            "energized"
                        } else {
                            "not energized"
                        }
                    )
                } else {
                    format!("{label}: no run enable terminal")
                },
            );
            add(
                3,
                if tripped {
                    CheckState::Fault
                } else {
                    CheckState::Ok
                },
                if tripped {
                    format!("{label}: faulted (fault relay closed)")
                } else {
                    format!("{label}: no fault")
                },
            );
            let id = definition.id.as_str();
            let seen = self
                .scene_def
                .installation
                .wiring
                .iter()
                .filter(|w| {
                    w.component.as_str() == id
                        && definition.kind.command_quantity(&w.port) == Some("command")
                })
                .any(|w| {
                    self.tests
                        .outputs
                        .get(&w.channel)
                        .is_some_and(|t| t.received)
                });
            add(
                4,
                if seen {
                    CheckState::Ok
                } else {
                    CheckState::Untested
                },
                format!(
                    "{label}: run command {}",
                    if commanded { "present" } else { "absent" }
                ),
            );
            let keypad = q("keypadReference");
            add(
                5,
                if keypad {
                    CheckState::Attention
                } else {
                    CheckState::Ok
                },
                format!(
                    "{label}: reference {} from {}",
                    percent(permille("referencePermille")),
                    if keypad {
                        "the keypad"
                    } else {
                        "the terminals"
                    }
                ),
            );
            let running = q("running");
            add(
                6,
                if commanded && remote && ready && !running {
                    CheckState::Attention
                } else if seen {
                    CheckState::Ok
                } else {
                    CheckState::Untested
                },
                format!(
                    "{label}: {} at {} speed",
                    if running { "running" } else { "stopped" },
                    percent(permille("speedPermille"))
                ),
            );
        }
        CheckCategory {
            id: "drives",
            title: "Drives",
            items: ITEMS
                .iter()
                .zip(results)
                .map(|((id, title, how), (states, findings))| {
                    let state = if states.iter().all(|s| *s == CheckState::NotApplicable) {
                        CheckState::NotApplicable
                    } else {
                        worst(states)
                    };
                    item(id, title, state, findings, how)
                })
                .collect(),
        }
    }

    fn process(&self) -> CheckCategory {
        let mut trips = Vec::new();
        let mut limits = Vec::new();
        for (index, component) in self.scene_def.components.iter().enumerate() {
            match component.kind {
                ComponentKind::MotorStarter { .. } => {
                    if is_on(self.scene.quantity(index, "tripped")) {
                        trips.push(format!("{}: overload relay tripped", component.label));
                    }
                }
                ComponentKind::Pump { .. } => {
                    if is_on(self.scene.quantity(index, "dry")) {
                        trips.push(format!("{}: running dry", component.label));
                    }
                }
                ComponentKind::Tank { .. } => {
                    for (quantity, text) in [
                        ("overflowing", "overflowing"),
                        ("highHigh", "above its high-high level"),
                        ("lowLow", "below its low-low level"),
                    ] {
                        if is_on(self.scene.quantity(index, quantity)) {
                            limits.push(format!("{}: {text}", component.label));
                        }
                    }
                }
                _ => {}
            }
        }
        let timeouts: Vec<String> = self
            .tests
            .outputs
            .iter()
            .filter(|(_, t)| {
                t.energized_us >= FEEDBACK_WAIT_US && t.feedback.values().any(|seen| !follows(seen))
            })
            .map(|(channel, _)| {
                format!(
                    "{}: energized without the expected feedback",
                    self.channel_name(*channel)
                )
            })
            .collect();
        let running = self.plc_mode() == PlcMode::Run;
        let sequence = item(
            "sequence",
            "Sequence",
            if running {
                CheckState::Ok
            } else {
                CheckState::Untested
            },
            vec![if running {
                format!("The PLC executes its program ({} scans)", self.plc_scans())
            } else {
                "The PLC does not execute a program (not in RUN)".into()
            }],
            "Program view: online values; Cause & effect: the last cycle.",
        );
        let alarms: Vec<String> = trips.iter().chain(&limits).cloned().collect();
        CheckCategory {
            id: "process",
            title: "Process",
            items: vec![
                item(
                    "interlocks",
                    "Interlocks",
                    if trips.is_empty() {
                        CheckState::Ok
                    } else {
                        CheckState::Fault
                    },
                    if trips.is_empty() {
                        vec!["No protective device has tripped".into()]
                    } else {
                        trips
                    },
                    "Process view: overload relays and protective devices.",
                ),
                item(
                    "permissives",
                    "Process limits",
                    if limits.is_empty() {
                        CheckState::Ok
                    } else {
                        CheckState::Attention
                    },
                    if limits.is_empty() {
                        vec!["The process is within its limits".into()]
                    } else {
                        limits
                    },
                    "Process view: levels and limit switches.",
                ),
                sequence,
                item(
                    "timeout",
                    "Timeouts",
                    if timeouts.is_empty() {
                        CheckState::Ok
                    } else {
                        CheckState::Fault
                    },
                    if timeouts.is_empty() {
                        vec!["No actuator waits for its feedback".into()]
                    } else {
                        timeouts
                    },
                    "Watch: feedback inputs after a command.",
                ),
                item(
                    "alarms",
                    "Alarms",
                    if alarms.is_empty() {
                        CheckState::Ok
                    } else {
                        CheckState::Attention
                    },
                    if alarms.is_empty() {
                        vec!["No process alarm condition".into()]
                    } else {
                        alarms
                    },
                    "Process view and Diagnostics: alarm conditions.",
                ),
            ],
        }
    }
}

/// One mapped PLC signal followed through the installation: the symbol, its configured
/// channel (stable id and address), what the channel reads or drives, and the device the
/// drawing connects to that channel with that device's own state.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalMapRow {
    pub tag: String,
    pub channel: ChannelRef,
    /// `<moduleId>.ch<n>`, `None` where no module is configured.
    pub channel_id: Option<String>,
    pub address: String,
    pub reading: Option<hardware_model::ChannelReading>,
    pub output: Option<bool>,
    /// The device and port connected to this channel on the drawing.
    pub drawn_device: Option<String>,
    pub drawn_port: Option<String>,
    /// What the drawn device does right now (actuated, commanded, contact closed, signal).
    pub device_state: Option<String>,
}

impl LabSession {
    /// The configured I/O mappings with their evidence, in mapping order.
    pub fn signal_map(&self) -> Vec<SignalMapRow> {
        let config = self.station.configuration();
        config
            .io_map
            .iter()
            .map(|mapping| {
                let wire = self
                    .scene_def
                    .installation
                    .wiring
                    .iter()
                    .find(|w| w.channel == mapping.channel);
                let device = wire.and_then(|w| {
                    let index = self.scene.index_of_str(w.component.as_str()).ok()?;
                    Some((index, w))
                });
                SignalMapRow {
                    tag: mapping.tag.to_string(),
                    channel: mapping.channel,
                    channel_id: config.channel_id(mapping.channel),
                    address: self.channel_address(mapping.channel),
                    reading: self.station.reading(mapping.channel),
                    output: self.station.output(mapping.channel),
                    drawn_device: device.map(|(i, _)| self.scene_def.components[i].label.clone()),
                    drawn_port: wire.map(|w| w.port.clone()),
                    device_state: device.map(|(i, w)| self.device_state(i, &w.port)),
                }
            })
            .collect()
    }

    /// A device's own state at one of its ports, as seen at the device.
    fn device_state(&self, index: usize, port: &str) -> String {
        let kind = &self.scene_def.components[index].kind;
        match kind.port_role(port) {
            Some(PortRole::Sensor) => {
                let actuated =
                    match actuation(kind, port).and_then(|q| self.scene.quantity(index, q)) {
                        Some(Quantity::Integer(p)) => port_position(kind, port) == Some(p),
                        other => is_on(other),
                    };
                if actuated { "actuated" } else { "not actuated" }.into()
            }
            Some(PortRole::Command) => {
                let on = kind
                    .command_quantity(port)
                    .is_some_and(|q| is_on(self.scene.quantity(index, q)));
                if on { "energized" } else { "not energized" }.into()
            }
            Some(PortRole::Feedback | PortRole::Status) => {
                let closed = self
                    .scene
                    .digital_output(index, port)
                    .is_ok_and(|f| f.conducting);
                if closed {
                    "contact closed"
                } else {
                    "contact open"
                }
                .into()
            }
            Some(PortRole::Measurement) => match self.scene.analog_output(index, port) {
                Ok(f) => match f.unit {
                    process_simulation::AnalogUnit::Microamp => {
                        format!("sends {}.{:02} mA", f.value / 1000, (f.value % 1000) / 10)
                    }
                    process_simulation::AnalogUnit::Millivolt => {
                        format!("sends {}.{:02} V", f.value / 1000, (f.value % 1000) / 10)
                    }
                },
                Err(_) => "no signal".into(),
            },
            None => "unknown".into(),
        }
    }
}
