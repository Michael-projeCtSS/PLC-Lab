//! The lab session: one deterministic world containing a scene, a station, a network
//! and one PLC participant, advanced cycle by cycle.
//!
//! One cycle (see ADR-0018):
//! 1. scan with the staged input image (already sampled at the cycle start);
//! 2. drive outputs at the cycle start (RUN: committed outputs, otherwise STOP behavior);
//! 3. advance the world by the cycle period, interleaving scheduled actions, network
//!    events and exact physics (skipped when the scan faulted);
//! 4. sample all inputs at the cycle end and stage them for the next scan;
//! 5. record one causal observation: scan inputs, committed values, applied commands,
//!    process result and staged next inputs.

use crate::{
    error::{BindingIssue, EngineError},
    faults,
    plc::{self, InputSource, LoadedProgram, PlcMode, PlcParticipant, ScanOutcome, TagRole},
    signals::{SignalId, SignalKind, SignalRegistry, SignalValue, ValueType, unit_of},
};
use hardware_model::{ChannelValue, DiagnosticKind, DiagnosticSource, StationRuntime};
use lab_model::{
    Identifier,
    exercise::{PlcModeRequest, ScenarioAction},
    fault::{FaultInjection, FaultSpec},
    hardware::{ChannelRef, ModuleClass, StationConfig},
    profile::{AnalogRepresentation, PlatformProfile},
    scene::SceneDefinition,
};
use network_core::{NetworkEvent, NetworkRuntime, ScanStatus, ScanValue};
use plc_types::{PlcValue, diagnostics::Diagnostic};
use process_simulation::{ProcessEventKind, Quantity, SceneRuntime};
use simulation_core::{Phase, Scheduler, WorldTime};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
};
use trace_core::{EventLog, RecordSeq, SignalTrace, TraceConfig, TraceEvent};

pub const LAYER_INPUTS: usize = 0;
pub const LAYER_COMMITTED: usize = 1;
pub const LAYER_APPLIED: usize = 2;
pub const LAYER_PROCESS: usize = 3;
pub const LAYER_NEXT: usize = 4;
pub const LAYERS: usize = 5;

pub type Layer = Vec<(SignalId, SignalValue)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionConfig {
    pub cycle_us: u64,
    pub operation_budget: u64,
    pub seed: u64,
    pub trace_bytes: usize,
}
impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cycle_us: 100_000,
            operation_budget: 10_000,
            seed: 1,
            trace_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    Scanned,
    Faulted,
    NotScanned(PlcMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleHeader {
    pub cycle: u64,
    pub start_us: u64,
    pub end_us: u64,
    pub segment: u32,
    pub task_time_us: u64,
    pub outcome: CycleOutcome,
}

/// Causal record of one cycle. `Output == f(scan_inputs)` holds within one observation.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleObservation {
    pub header: CycleHeader,
    pub scan_inputs: Layer,
    pub committed: Layer,
    pub applied: Layer,
    pub process_after: Layer,
    pub staged_next: Layer,
    pub fault: Option<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeReason {
    Switch,
    RuntimeFault,
    Download,
    MemoryReset,
    WorldReset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorAction {
    Press { component: String },
    Release { component: String },
    SetSelector { component: String, position: u8 },
    SetPresence { component: String, present: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LabEventKind {
    ModeChanged {
        from: PlcMode,
        to: PlcMode,
        reason: ModeReason,
    },
    PlcFault {
        diagnostic: Diagnostic,
    },
    ProgramDownloaded {
        name: String,
        revision: u64,
    },
    HardwareDownloaded,
    WorldReset,
    Process {
        component: String,
        kind: ProcessEventKind,
    },
    Diagnostic {
        entry: hardware_model::DiagnosticEntry,
    },
    Network {
        event: NetworkEvent,
    },
    Operator {
        action: OperatorAction,
    },
    Force {
        signal: SignalId,
        value: Option<SignalValue>,
    },
    Repair {
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabEvent {
    pub time_us: u64,
    pub cycle: u64,
    pub kind: LabEventKind,
}
impl TraceEvent for LabEvent {
    fn encoded_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + match &self.kind {
                LabEventKind::PlcFault { diagnostic } => diagnostic.message().len(),
                LabEventKind::ProgramDownloaded { name, .. } => name.len(),
                LabEventKind::Process { component, .. } => component.len(),
                LabEventKind::Repair { label } => label.len(),
                _ => 0,
            }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WorldEvent {
    Action(ScenarioAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadReport {
    pub program_name: String,
    pub warnings: Vec<BindingIssue>,
}

/// Reads scene quantities for network servers at the instant a request is served.
struct SceneQuantities<'a>(&'a SceneRuntime);
impl network_core::QuantitySource for SceneQuantities<'_> {
    fn quantity(&self, component: &str, name: &str) -> Option<i64> {
        let index = self.0.index_of_str(component).ok()?;
        self.0.quantity(index, name).map(Quantity::as_i64)
    }
}

pub struct LabSession {
    pub(crate) config: SessionConfig,
    pub(crate) scene_def: SceneDefinition,
    pub(crate) profile: PlatformProfile,
    pub(crate) scene: SceneRuntime,
    pub(crate) station: StationRuntime,
    pub(crate) network: NetworkRuntime,
    pub(crate) scheduler: Scheduler<WorldEvent>,
    pub(crate) plc: PlcParticipant,
    pub(crate) signals: SignalRegistry,
    pub(crate) physical: Vec<(usize, &'static str, SignalId)>,
    pub(crate) channels: Vec<(ChannelRef, SignalId)>,
    pub(crate) channel_values: BTreeMap<SignalId, SignalValue>,
    pub(crate) trace: SignalTrace<CycleHeader, SignalValue>,
    pub(crate) events: EventLog<LabEvent>,
    pub(crate) cycle: u64,
    pub(crate) active_faults: BTreeMap<Identifier, FaultSpec>,
    pub(crate) diagnostics_seen: u64,
    pub(crate) last: Option<CycleObservation>,
    pub(crate) previous: Option<CycleObservation>,
    /// I/O tests performed in this session, for the commissioning checklist.
    pub(crate) tests: crate::commissioning::IoTests,
}

fn quantity_value(q: Quantity) -> SignalValue {
    match q {
        Quantity::Bool(b) => SignalValue::Bool(b),
        Quantity::Integer(i) => SignalValue::Count(i),
    }
}

impl LabSession {
    pub fn new(
        scene_def: SceneDefinition,
        station: StationConfig,
        profile: PlatformProfile,
        config: SessionConfig,
    ) -> Result<Self, EngineError> {
        lab_model::validate_scene(&scene_def).map_err(|e| EngineError::Scene(e.to_string()))?;
        let scene = SceneRuntime::new(&scene_def, config.seed)?;
        let station_runtime = StationRuntime::new(
            station.clone(),
            scene_def.installation.clone(),
            profile.clone(),
        )?;
        let mut network = NetworkRuntime::new(scene_def.network.as_ref());
        network.configure(
            station.network.interface.clone(),
            &station.network.modbus_tcp,
        );
        let mut session = Self {
            plc: PlcParticipant::new(config.cycle_us, config.operation_budget)?,
            trace: new_trace(config.trace_bytes),
            events: EventLog::new(
                NonZeroUsize::new(20_000).expect("positive"),
                4 * 1024 * 1024,
            ),
            config,
            scene_def,
            profile,
            scene,
            station: station_runtime,
            network,
            scheduler: Scheduler::new(),
            signals: SignalRegistry::default(),
            physical: Vec::new(),
            channels: Vec::new(),
            channel_values: BTreeMap::new(),
            cycle: 0,
            active_faults: BTreeMap::new(),
            diagnostics_seen: 0,
            last: None,
            previous: None,
            tests: Default::default(),
        };
        session.register_physical();
        session.register_channels();
        session.sample()?;
        session.drive()?;
        Ok(session)
    }

    pub fn now(&self) -> WorldTime {
        self.scene.now()
    }
    pub fn cycle(&self) -> u64 {
        self.cycle
    }
    pub fn config(&self) -> SessionConfig {
        self.config
    }
    pub fn profile(&self) -> &PlatformProfile {
        &self.profile
    }
    pub fn signals(&self) -> &SignalRegistry {
        &self.signals
    }
    pub fn scene(&self) -> &SceneRuntime {
        &self.scene
    }
    pub fn station(&self) -> &StationRuntime {
        &self.station
    }
    pub fn network(&self) -> &NetworkRuntime {
        &self.network
    }
    pub fn events(&self) -> &EventLog<LabEvent> {
        &self.events
    }
    pub fn latest(&self) -> Option<&CycleObservation> {
        self.last.as_ref()
    }
    pub fn plc_mode(&self) -> PlcMode {
        self.plc.mode
    }
    pub fn plc_switch(&self) -> PlcModeRequest {
        self.plc.switch
    }
    pub fn plc_fault(&self) -> Option<&Diagnostic> {
        self.plc.fault.as_ref()
    }
    pub fn plc_segment(&self) -> u32 {
        self.plc.segment
    }
    pub fn plc_scans(&self) -> u64 {
        self.plc.scans
    }
    pub fn plc_task_time_us(&self) -> u64 {
        self.plc.task_time()
    }
    pub fn program(&self) -> Option<(&str, u64, &str)> {
        self.plc
            .program
            .as_ref()
            .map(|p| (p.name.as_str(), p.revision, p.source.as_str()))
    }
    pub fn tags(&self) -> &[plc::TagBinding] {
        self.plc.tags()
    }
    pub fn blocks(&self) -> &[plc::BlockBinding] {
        self.plc.blocks()
    }
    pub fn forces(&self) -> &BTreeMap<SignalId, SignalValue> {
        &self.plc.forces
    }
    pub fn active_fault_ids(&self) -> impl Iterator<Item = &Identifier> {
        self.active_faults.keys()
    }
    pub fn trace_stats(&self) -> trace_core::TraceStats {
        self.trace.stats()
    }

    fn register_physical(&mut self) {
        self.physical.clear();
        for index in 0..self.scene.component_count() {
            let id = self
                .scene
                .component_id(index)
                .expect("component index")
                .to_string();
            let label = self
                .scene
                .component_label(index)
                .unwrap_or_default()
                .to_owned();
            for (name, value) in self.scene.quantities(index) {
                let value_type = match value {
                    Quantity::Bool(_) => ValueType::Bool,
                    Quantity::Integer(_) => ValueType::Count,
                };
                let signal = self.signals.ensure(
                    format!("process.{id}.{name}"),
                    format!("{label} {name}"),
                    SignalKind::Physical,
                    value_type,
                    unit_of(name),
                );
                self.physical.push((index, name, signal));
            }
        }
    }

    fn register_channels(&mut self) {
        self.channels.clear();
        self.channel_values.clear();
        let real = self.profile.analog == AnalogRepresentation::EngineeringUnits;
        let config = self.station.configuration().clone();
        for module in &config.modules {
            let value_type = match module.module.class() {
                ModuleClass::DigitalInput { .. } | ModuleClass::DigitalOutput { .. } => {
                    ValueType::Bool
                }
                ModuleClass::AnalogInput { .. } | ModuleClass::AnalogOutput { .. } => {
                    if real {
                        ValueType::Real
                    } else {
                        ValueType::Int
                    }
                }
                _ => continue,
            };
            for channel in 0..module.module.channel_count() {
                let c = ChannelRef {
                    rack: module.position.rack,
                    slot: module.position.slot,
                    channel,
                };
                let signal = self.signals.ensure(
                    format!("io.{c}"),
                    c.to_string(),
                    SignalKind::Channel,
                    value_type,
                    None,
                );
                self.channels.push((c, signal));
            }
        }
    }

    /// Sample every input at the current instant and stage it for the next scan.
    pub(crate) fn sample(&mut self) -> Result<(), EngineError> {
        let now = self.now();
        let readings = self.station.sample_inputs(&self.scene, now).clone();
        for (channel, signal) in &self.channels {
            if let Some(r) = readings.get(channel) {
                self.channel_values.insert(*signal, channel_signal(r.value));
            } else if let Some(out) = self.station.output(*channel) {
                self.channel_values.insert(*signal, SignalValue::Bool(out));
            }
        }
        let mut staged = Vec::new();
        for tag in self.plc.tags() {
            if let Some(forced) = self.plc.forces.get(&tag.signal) {
                if tag.role == TagRole::Input {
                    staged.push((tag.symbol, forced.to_plc()));
                }
                continue;
            }
            let value = match tag.source {
                Some(InputSource::Channel(c)) => readings.get(&c).map(|r| match r.value {
                    ChannelValue::Bool(b) => PlcValue::Bool(b),
                    ChannelValue::Int(i) => PlcValue::Int(i),
                    ChannelValue::Real(v) => PlcValue::real(v).unwrap_or(PlcValue::Int(0)),
                }),
                Some(InputSource::Modbus { scan, mapping }) => {
                    match self.network.value(scan, mapping) {
                        Some(ScanValue::Int(i)) => Some(PlcValue::Int(i)),
                        Some(ScanValue::Real(r)) => PlcValue::real(r).ok(),
                        Some(ScanValue::NotFinite) | None => None,
                    }
                }
                Some(InputSource::ModbusStatus { scan }) => Some(PlcValue::Bool(
                    self.network.scan_status(scan) == Some(ScanStatus::Ok),
                )),
                None => None,
            };
            if let Some(value) = value
                && value.plc_type() == tag.plc_type
            {
                staged.push((tag.symbol, Some(value)));
            }
        }
        for (symbol, value) in staged {
            if let Some(value) = value {
                self.plc.stage(symbol, value)?;
            }
        }
        // Network connection diagnostics share the CPU diagnostic buffer.
        for scan in 0..self.network.scan_count() {
            let status = self.network.scan_status(scan).unwrap_or(ScanStatus::Idle);
            let source = DiagnosticSource::Connection(u16::try_from(scan).unwrap_or(u16::MAX));
            for (kind, active) in [
                (
                    DiagnosticKind::ConnectionTimeout,
                    status == ScanStatus::Timeout,
                ),
                (DiagnosticKind::NoRoute, status == ScanStatus::NoRoute),
                (
                    DiagnosticKind::ConnectionRefused,
                    status == ScanStatus::ConnectionRefused,
                ),
            ] {
                self.station
                    .set_external_diagnostic(source, kind, active, now);
            }
            for code in [1u8, 2, 3, 4, 0x0A, 0x0B] {
                self.station.set_external_diagnostic(
                    source,
                    DiagnosticKind::ModbusException(code),
                    status == ScanStatus::Exception(code),
                    now,
                );
            }
        }
        let forced = !self.plc.forces.is_empty();
        self.station.set_external_diagnostic(
            DiagnosticSource::Cpu,
            DiagnosticKind::ForceActive,
            forced,
            now,
        );
        self.station.set_external_diagnostic(
            DiagnosticSource::Cpu,
            DiagnosticKind::PlcFault,
            self.plc.mode == PlcMode::Fault,
            now,
        );
        self.collect_diagnostics();
        Ok(())
    }

    fn collect_diagnostics(&mut self) {
        let total = self.station.diagnostics().total();
        let new =
            usize::try_from(total.saturating_sub(self.diagnostics_seen)).unwrap_or(usize::MAX);
        let entries: Vec<_> = self
            .station
            .diagnostics()
            .entries()
            .rev()
            .take(new)
            .copied()
            .collect();
        for entry in entries.into_iter().rev() {
            self.events.append(LabEvent {
                time_us: entry.time.as_micros(),
                cycle: self.cycle,
                kind: LabEventKind::Diagnostic { entry },
            });
        }
        self.diagnostics_seen = total;
    }

    /// Drive output channels from the PLC (RUN) or the configured STOP behavior.
    pub(crate) fn drive(&mut self) -> Result<(), EngineError> {
        let running = self.plc.mode == PlcMode::Run;
        let mut commands = BTreeMap::new();
        let mut analog = BTreeMap::new();
        if let Some(snapshot) = self.plc.snapshot() {
            for tag in self.plc.tags() {
                let Some(channel) = tag.sink else { continue };
                let forced = self.plc.forces.get(&tag.signal).copied();
                match (forced, snapshot.committed().get(&tag.symbol).copied()) {
                    (Some(SignalValue::Bool(b)), _) | (None, Some(PlcValue::Bool(b))) => {
                        commands.insert(channel, b);
                    }
                    (Some(SignalValue::Int(i)), _) | (None, Some(PlcValue::Int(i))) => {
                        analog.insert(channel, hardware_model::analog::AnalogRaw::Int(i));
                    }
                    (Some(SignalValue::Real(r)), _) => {
                        analog.insert(channel, hardware_model::analog::AnalogRaw::Real(r));
                    }
                    (None, Some(PlcValue::Real(r))) => {
                        analog.insert(channel, hardware_model::analog::AnalogRaw::Real(r.get()));
                    }
                    _ => {}
                }
            }
        }
        self.station
            .drive_outputs(&commands, running, &mut self.scene)?;
        self.station.drive_analog(&analog, running);
        for (channel, signal) in &self.channels {
            if let Some(out) = self.station.output(*channel) {
                self.channel_values.insert(*signal, SignalValue::Bool(out));
            }
        }
        Ok(())
    }

    fn plc_layer(&self, image: Image) -> Layer {
        let Some(snapshot) = self.plc.snapshot() else {
            return Vec::new();
        };
        let mut layer: Layer = Vec::new();
        for tag in self.plc.tags() {
            let map = match (image, tag.role) {
                (Image::Scan, TagRole::Input) => snapshot.input_image(),
                (Image::Staged, TagRole::Input) => snapshot.staged_inputs(),
                (Image::Committed, TagRole::Output | TagRole::Local) => snapshot.committed(),
                _ => continue,
            };
            if let Some(v) = map.get(&tag.symbol) {
                layer.push((tag.signal, SignalValue::from_plc(*v)));
            }
        }
        if image == Image::Committed {
            for block in self.plc.blocks() {
                if let Some(state) = snapshot.function_blocks().get(&block.instance) {
                    for (name, value) in state.outputs() {
                        if let Some((_, signal)) = block.outputs.iter().find(|(n, _)| *n == name) {
                            layer.push((*signal, SignalValue::from_plc(value)));
                        }
                    }
                }
            }
        }
        layer.sort_by_key(|(s, _)| *s);
        layer
    }

    fn process_layer(&self) -> Layer {
        let mut layer: Layer = self
            .physical
            .iter()
            .filter_map(|(index, name, signal)| {
                self.scene
                    .quantity(*index, name)
                    .map(|q| (*signal, quantity_value(q)))
            })
            .collect();
        for (_, signal) in &self.channels {
            if let Some(v) = self.channel_values.get(signal) {
                layer.push((*signal, *v));
            }
        }
        layer.sort_by_key(|(s, _)| *s);
        layer
    }

    fn applied_layer(&self) -> Layer {
        let mut layer: Layer = self
            .channels
            .iter()
            .filter_map(|(c, s)| self.station.output(*c).map(|v| (*s, SignalValue::Bool(v))))
            .collect();
        for (index, name, signal) in &self.physical {
            if self.scene.is_commanded_quantity(*index, name)
                && let Some(q) = self.scene.quantity(*index, name)
            {
                layer.push((*signal, quantity_value(q)));
            }
        }
        layer.sort_by_key(|(s, _)| *s);
        layer
    }

    /// Advance the world, interleaving scheduled actions, network events and physics.
    fn advance_world(&mut self, end: WorldTime) -> Result<(), EngineError> {
        loop {
            let scheduled = self.scheduler.peek_key().filter(|k| k.time <= end);
            let network = self.network.next_event_time().filter(|t| *t <= end);
            let take_scheduled = match (scheduled, network) {
                (Some(k), Some(t)) => k.time < t || (k.time == t && k.phase < Phase::Network),
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => break,
            };
            if take_scheduled {
                let (key, WorldEvent::Action(action)) = self
                    .scheduler
                    .pop_until(end)
                    .ok_or_else(|| EngineError::Internal("scheduled event vanished".into()))?;
                self.scene.advance_to(key.time)?;
                self.apply_action(&action)?;
            } else {
                let at = network
                    .ok_or_else(|| EngineError::Internal("network event vanished".into()))?;
                self.scene.advance_to(at)?;
                let source = SceneQuantities(&self.scene);
                self.network.process_next(at, &source);
            }
        }
        self.scene.advance_to(end)?;
        self.network.advance_to(end);
        self.scheduler
            .advance_to(end)
            .map_err(|e| EngineError::Internal(e.to_string()))?;
        Ok(())
    }

    fn drain_world_events(&mut self) {
        for e in self.scene.drain_events() {
            self.events.append(LabEvent {
                time_us: e.time.as_micros(),
                cycle: self.cycle,
                kind: LabEventKind::Process {
                    component: e.component.to_string(),
                    kind: e.kind,
                },
            });
        }
        for e in self.network.drain_events() {
            self.events.append(LabEvent {
                time_us: e.time.as_micros(),
                cycle: self.cycle,
                kind: LabEventKind::Network { event: e },
            });
        }
    }

    fn note_mode(&mut self, change: Option<(PlcMode, PlcMode)>, reason: ModeReason) {
        if let Some((from, to)) = change {
            self.events.append(LabEvent {
                time_us: self.now().as_micros(),
                cycle: self.cycle,
                kind: LabEventKind::ModeChanged { from, to, reason },
            });
        }
    }

    /// Execute one PLC cycle and advance the world by one cycle period.
    pub fn step_cycle(&mut self) -> Result<CycleObservation, EngineError> {
        let start = self.now();
        let task_time = self.plc.task_time();
        let segment = self.plc.segment;
        let outcome = self.plc.scan()?;
        let (outcome, fault) = match outcome {
            ScanOutcome::Completed => (CycleOutcome::Scanned, None),
            ScanOutcome::Faulted(d) => {
                self.events.append(LabEvent {
                    time_us: start.as_micros(),
                    cycle: self.cycle,
                    kind: LabEventKind::PlcFault {
                        diagnostic: d.clone(),
                    },
                });
                self.note_mode(
                    Some((PlcMode::Run, PlcMode::Fault)),
                    ModeReason::RuntimeFault,
                );
                (CycleOutcome::Faulted, Some(d))
            }
            ScanOutcome::NotScanned => (CycleOutcome::NotScanned(self.plc.mode), None),
        };
        let scan_inputs = self.plc_layer(if matches!(outcome, CycleOutcome::NotScanned(_)) {
            Image::Staged
        } else {
            Image::Scan
        });
        self.drive()?;
        let committed = self.plc_layer(Image::Committed);
        let applied = self.applied_layer();
        let end = if outcome == CycleOutcome::Faulted {
            start
        } else {
            start
                .checked_add_micros(self.config.cycle_us)
                .ok_or_else(|| EngineError::Internal("world time overflow".into()))?
        };
        if end > start {
            self.advance_world(end)?;
        }
        self.drain_world_events();
        self.sample()?;
        let mut tests = std::mem::take(&mut self.tests);
        tests.observe(self);
        self.tests = tests;
        let observation = CycleObservation {
            header: CycleHeader {
                cycle: self.cycle,
                start_us: start.as_micros(),
                end_us: end.as_micros(),
                segment,
                task_time_us: task_time,
                outcome,
            },
            scan_inputs,
            committed,
            applied,
            process_after: self.process_layer(),
            staged_next: self.plc_layer(Image::Staged),
            fault,
        };
        self.trace
            .append(
                observation.header,
                vec![
                    observation
                        .scan_inputs
                        .iter()
                        .map(|(s, v)| (s.0, *v))
                        .collect(),
                    observation
                        .committed
                        .iter()
                        .map(|(s, v)| (s.0, *v))
                        .collect(),
                    observation.applied.iter().map(|(s, v)| (s.0, *v)).collect(),
                    observation
                        .process_after
                        .iter()
                        .map(|(s, v)| (s.0, *v))
                        .collect(),
                    observation
                        .staged_next
                        .iter()
                        .map(|(s, v)| (s.0, *v))
                        .collect(),
                ],
            )
            .map_err(|e| EngineError::Internal(e.to_string()))?;
        self.cycle += 1;
        self.previous = self.last.replace(observation.clone());
        Ok(observation)
    }

    /// Schedule a scenario action at an absolute world time (headless assessment).
    pub fn schedule(&mut self, at: WorldTime, action: ScenarioAction) -> Result<(), EngineError> {
        self.scheduler
            .schedule(at, Phase::Stimulus, 0, WorldEvent::Action(action))
            .map(|_| ())
            .map_err(|e| EngineError::InvalidAction(e.to_string()))
    }

    fn apply_action(&mut self, action: &ScenarioAction) -> Result<(), EngineError> {
        match action {
            ScenarioAction::Press { component } => self.scene.press(component.as_str(), true)?,
            ScenarioAction::Release { component } => self.scene.press(component.as_str(), false)?,
            ScenarioAction::SetSelector {
                component,
                position,
            } => self.scene.set_selector(component.as_str(), *position)?,
            ScenarioAction::SetPresence { component, present } => {
                self.scene.set_presence(component.as_str(), *present)?
            }
            ScenarioAction::SetPlcMode { mode } => {
                self.plc.switch = *mode;
                let change = self.plc.settle_mode();
                self.note_mode(change, ModeReason::Switch);
                if change.is_some() {
                    self.drive()?;
                }
            }
            ScenarioAction::InjectFault { fault } => self.inject_fault(fault)?,
            ScenarioAction::ClearFault { id } => {
                self.clear_fault(id)?;
            }
        }
        Ok(())
    }

    /// An operator action at the current instant (between cycles). Inputs are re-sampled
    /// so the next scan sees the change; the previous observation is not modified.
    pub fn operate(&mut self, action: OperatorAction) -> Result<(), EngineError> {
        match &action {
            OperatorAction::Press { component } => self.scene.press(component, true)?,
            OperatorAction::Release { component } => self.scene.press(component, false)?,
            OperatorAction::SetSelector {
                component,
                position,
            } => self.scene.set_selector(component, *position)?,
            OperatorAction::SetPresence { component, present } => {
                self.scene.set_presence(component, *present)?
            }
        }
        self.events.append(LabEvent {
            time_us: self.now().as_micros(),
            cycle: self.cycle,
            kind: LabEventKind::Operator { action },
        });
        self.sample()
    }

    pub fn set_plc_switch(&mut self, mode: PlcModeRequest) -> Result<PlcMode, EngineError> {
        self.plc.switch = mode;
        let change = self.plc.settle_mode();
        self.note_mode(change, ModeReason::Switch);
        if change.is_some() {
            self.sample()?;
            self.drive()?;
        }
        if mode == PlcModeRequest::Run {
            if self.plc.mode == PlcMode::Fault {
                return Err(EngineError::PlcFaulted);
            }
            if self.plc.program.is_none() {
                return Err(EngineError::NoProgram);
            }
        }
        Ok(self.plc.mode)
    }

    /// Memory reset: clears the fault and all non-retentive memory, then settles the mode.
    pub fn reset_plc(&mut self) -> Result<PlcMode, EngineError> {
        let before = self.plc.mode;
        self.plc.memory_reset();
        let settled = self.plc.settle_mode();
        let to = settled.map_or(self.plc.mode, |(_, to)| to);
        if before != to {
            self.note_mode(Some((before, to)), ModeReason::MemoryReset);
        }
        self.sample()?;
        self.drive()?;
        Ok(self.plc.mode)
    }

    /// Compile and check a program against a station without changing the session.
    pub fn check(
        &self,
        source: &str,
        station: Option<&StationConfig>,
    ) -> Result<Vec<BindingIssue>, EngineError> {
        let compiled = plc::compile(source).map_err(EngineError::Compile)?;
        let station = station.unwrap_or(self.station.configuration());
        let mut scratch = self.signals.clone();
        plc::link(&compiled, station, &self.profile, &mut scratch)
            .map(|(_, _, warnings)| warnings)
            .map_err(EngineError::Binding)
    }

    /// Download a program (and optionally a hardware configuration) into the CPU.
    /// The world keeps running state; the CPU performs a warm restart if its switch is RUN.
    pub fn download(
        &mut self,
        source: &str,
        revision: u64,
        station: Option<StationConfig>,
    ) -> Result<DownloadReport, EngineError> {
        let compiled = plc::compile(source).map_err(EngineError::Compile)?;
        let target_station = station
            .clone()
            .unwrap_or_else(|| self.station.configuration().clone());
        lab_model::validate_station(&target_station)
            .map_err(|e| EngineError::Hardware(e.to_string()))?;
        if target_station.cpu != self.scene_def.installation.cpu {
            return Err(EngineError::Hardware(
                hardware_model::HardwareError::CpuMismatch {
                    configured: target_station.cpu,
                    installed: self.scene_def.installation.cpu,
                }
                .to_string(),
            ));
        }
        let mut registry = self.signals.clone();
        let (tags, blocks, warnings) =
            plc::link(&compiled, &target_station, &self.profile, &mut registry)
                .map_err(EngineError::Binding)?;
        let validated = compiled.validated().map_err(EngineError::Compile)?;
        let plan = plc_runtime::lower(&validated)
            .map_err(|e| EngineError::Compile(vec![e.diagnostic().clone()]))?;
        let now = self.now();
        if let Some(station) = station {
            self.station.reconfigure(station.clone(), now)?;
            self.network.configure(
                station.network.interface.clone(),
                &station.network.modbus_tcp,
            );
            self.signals = registry;
            self.register_channels();
            self.events.append(LabEvent {
                time_us: now.as_micros(),
                cycle: self.cycle,
                kind: LabEventKind::HardwareDownloaded,
            });
        } else {
            self.signals = registry;
        }
        let keep: BTreeSet<SignalId> = tags
            .iter()
            .map(|t| t.signal)
            .chain(
                blocks
                    .iter()
                    .flat_map(|b| b.outputs.iter().map(|(_, s)| *s)),
            )
            .collect();
        self.signals.retire(|d| {
            matches!(
                d.kind,
                SignalKind::PlcInput
                    | SignalKind::PlcOutput
                    | SignalKind::PlcLocal
                    | SignalKind::BlockOutput
            ) && !keep.contains(&d.id)
        });
        self.plc.forces.retain(|signal, _| keep.contains(signal));
        let before = self.plc.mode;
        let name = compiled.program_name().to_owned();
        self.plc.program = Some(LoadedProgram {
            name: name.clone(),
            revision,
            source: source.to_owned(),
            runtime: plc_runtime::Runtime::new(plan, self.plc.config),
            tags,
            blocks,
        });
        self.plc.fault = None;
        self.plc.mode = PlcMode::Stop;
        self.plc.segment += 1;
        self.plc.settle_mode();
        if before != self.plc.mode {
            self.note_mode(Some((before, self.plc.mode)), ModeReason::Download);
        }
        self.events.append(LabEvent {
            time_us: now.as_micros(),
            cycle: self.cycle,
            kind: LabEventKind::ProgramDownloaded {
                name: name.clone(),
                revision,
            },
        });
        self.sample()?;
        self.drive()?;
        Ok(DownloadReport {
            program_name: name,
            warnings,
        })
    }

    /// Restore the scene's initial physical state and power-cycle the CPU, keeping the
    /// downloaded program, the hardware configuration and the mode switch position.
    pub fn reset_world(&mut self) -> Result<(), EngineError> {
        let station = self.station.configuration().clone();
        let program = self
            .plc
            .program
            .as_ref()
            .map(|p| (p.source.clone(), p.revision));
        let switch = self.plc.switch;
        let faults: Vec<FaultInjection> = self
            .active_faults
            .iter()
            .map(|(id, fault)| FaultInjection {
                id: id.clone(),
                fault: fault.clone(),
            })
            .collect();
        let mut fresh = Self::new(
            self.scene_def.clone(),
            station,
            self.profile.clone(),
            self.config,
        )?;
        // Keep signal identities stable across world resets.
        fresh.signals = self.signals.clone();
        fresh.register_physical();
        fresh.register_channels();
        for fault in &faults {
            fresh.inject_fault(fault)?;
        }
        fresh.plc.switch = switch;
        if let Some((source, revision)) = program {
            fresh.download(&source, revision, None)?;
        }
        fresh.plc.settle_mode();
        fresh.events = std::mem::replace(
            &mut self.events,
            EventLog::new(NonZeroUsize::new(1).expect("positive"), 0),
        );
        fresh.events.append(LabEvent {
            time_us: 0,
            cycle: 0,
            kind: LabEventKind::WorldReset,
        });
        fresh.sample()?;
        fresh.drive()?;
        *self = fresh;
        Ok(())
    }

    pub fn force(
        &mut self,
        signal: SignalId,
        value: Option<SignalValue>,
    ) -> Result<(), EngineError> {
        let tag = self
            .plc
            .tags()
            .iter()
            .find(|t| t.signal == signal)
            .ok_or_else(|| EngineError::UnknownSignal(format!("{}", signal.0)))?;
        if tag.role == TagRole::Local {
            return Err(EngineError::InvalidAction(
                "Only inputs and outputs can be forced.".into(),
            ));
        }
        match value {
            Some(v) => {
                if v.to_plc().map(|p| p.plc_type()) != Some(tag.plc_type) {
                    return Err(EngineError::InvalidAction(
                        "The forced value has the wrong type.".into(),
                    ));
                }
                self.plc.forces.insert(signal, v);
            }
            None => {
                self.plc.forces.remove(&signal);
            }
        }
        self.events.append(LabEvent {
            time_us: self.now().as_micros(),
            cycle: self.cycle,
            kind: LabEventKind::Force { signal, value },
        });
        self.sample()?;
        self.drive()
    }

    pub fn inject_fault(&mut self, injection: &FaultInjection) -> Result<(), EngineError> {
        faults::apply(self, &injection.fault, true)?;
        self.active_faults
            .insert(injection.id.clone(), injection.fault.clone());
        self.sample()?;
        self.drive()
    }

    pub fn clear_fault(&mut self, id: &Identifier) -> Result<bool, EngineError> {
        let Some(fault) = self.active_faults.remove(id) else {
            return Ok(false);
        };
        faults::apply(self, &fault, false)?;
        self.sample()?;
        self.drive()?;
        Ok(true)
    }

    /// Apply a physical repair visible to the student (logged without revealing the fault).
    pub fn repair(&mut self, label: &str, clears: &[Identifier]) -> Result<usize, EngineError> {
        // The installation changed: earlier I/O tests no longer describe it.
        self.tests = Default::default();
        let mut cleared = 0;
        for id in clears {
            if self.clear_fault(id)? {
                cleared += 1;
            }
        }
        self.events.append(LabEvent {
            time_us: self.now().as_micros(),
            cycle: self.cycle,
            kind: LabEventKind::Repair {
                label: label.to_owned(),
            },
        });
        Ok(cleared)
    }

    pub(crate) fn trace(&self) -> &SignalTrace<CycleHeader, SignalValue> {
        &self.trace
    }
    pub(crate) fn seq_of(&self, cycle: u64) -> RecordSeq {
        RecordSeq(cycle + 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Image {
    Scan,
    Staged,
    Committed,
}

fn channel_signal(value: ChannelValue) -> SignalValue {
    match value {
        ChannelValue::Bool(b) => SignalValue::Bool(b),
        ChannelValue::Int(i) => SignalValue::Int(i),
        ChannelValue::Real(r) => SignalValue::Real(r),
    }
}

fn new_trace(bytes: usize) -> SignalTrace<CycleHeader, SignalValue> {
    SignalTrace::new(TraceConfig {
        layers: LAYERS,
        byte_budget: bytes.max(64 * 1024),
        keyframe_interval: NonZeroUsize::new(64).expect("positive"),
    })
}
