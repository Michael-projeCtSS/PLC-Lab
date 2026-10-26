//! Station hardware behavior: configured versus installed modules, field wiring,
//! channel transfer between the physical process and PLC I/O tags, module diagnostics,
//! LEDs and measurements. Vendor products are presented through the platform profile;
//! behavior depends only on generic module types and profile behavior selections.

pub mod address;
pub mod analog;
mod diagnostics;

pub use analog::{AnalogCondition, AnalogRaw, AnalogReading};
pub use diagnostics::{DiagnosticBuffer, DiagnosticEntry, DiagnosticEvent, DiagnosticKind, DiagnosticSource};

use lab_model::{
    hardware::{
        ChannelMode, ChannelRef, DigitalInputStyle, InstalledHardware, ModuleClass, ModuleType,
        OutputStopBehavior, SlotRef, StationConfig,
    },
    profile::PlatformProfile,
    scene::OutputStyle,
};
use process_simulation::{AnalogField, SceneError, SceneRuntime};
use simulation_core::WorldTime;
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareError {
    Invalid(String),
    CpuMismatch { configured: ModuleType, installed: ModuleType },
    Scene(SceneError),
}
impl std::fmt::Display for HardwareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(m) => f.write_str(m),
            Self::CpuMismatch { configured, installed } => write!(
                f,
                "The configured CPU ({}) does not match the installed CPU ({}). The hardware configuration was not downloaded.",
                configured.generic_name(),
                installed.generic_name()
            ),
            Self::Scene(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for HardwareError {}
impl From<SceneError> for HardwareError {
    fn from(e: SceneError) -> Self {
        Self::Scene(e)
    }
}

/// Physical hardware faults (installation and wiring), applied by the engine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum HardwareFault {
    OpenCircuit(ChannelRef),
    ShortToSupply(ChannelRef),
    SwappedWires(ChannelRef, ChannelRef),
    ModuleMissing(SlotRef),
    WrongModule(SlotRef, ModuleType),
    WrongSlot(SlotRef, SlotRef),
    FieldPowerMissing(SlotRef),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModuleStatus {
    Ok,
    /// Configured but nothing is plugged in at this position.
    Missing,
    /// Another module type is plugged in.
    Mismatch(ModuleType),
    /// Plugged in, but its load/field supply is missing.
    NoFieldPower,
    /// Plugged in but not part of the configuration.
    Unconfigured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    Good,
    NotMapped,
    ChannelDisabled,
    ModuleFault,
    NoFieldPower,
    WireBreak,
    Overflow,
    Underflow,
    Overrange,
    Underrange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelValue {
    Bool(bool),
    Int(i16),
    Real(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelReading {
    pub value: ChannelValue,
    pub quality: Quality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measurement {
    /// DC volts at the terminal against M, millivolts.
    Millivolts(i64),
    /// Loop current through the terminal, microamperes.
    Microamps(i64),
    NoModule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedState {
    Off,
    Green,
    GreenFlashing,
    Red,
    RedFlashing,
    Amber,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelView {
    pub channel: u8,
    pub mode: ChannelMode,
    pub tag: Option<String>,
    pub address: String,
    pub reading: Option<ChannelReading>,
    pub output: Option<bool>,
    pub led: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleView {
    pub position: SlotRef,
    pub configured: Option<ModuleType>,
    pub installed: Option<ModuleType>,
    pub status: ModuleStatus,
    pub product: Option<String>,
    pub status_led: LedState,
    pub channels: Vec<ChannelView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Installed {
    module: ModuleType,
    powered: bool,
    /// Where the module's terminals were originally wired (front connector follows the module).
    wired_as: SlotRef,
}

#[derive(Debug, Clone)]
pub struct StationRuntime {
    configured: StationConfig,
    installation: InstalledHardware,
    profile: PlatformProfile,
    faults: BTreeSet<HardwareFault>,
    statuses: BTreeMap<SlotRef, ModuleStatus>,
    readings: BTreeMap<ChannelRef, ChannelReading>,
    outputs: BTreeMap<ChannelRef, bool>,
    last_run_outputs: BTreeMap<ChannelRef, bool>,
    buffer: DiagnosticBuffer,
    active: BTreeSet<(DiagnosticSource, DiagnosticKind)>,
}

impl StationRuntime {
    pub fn new(
        configured: StationConfig,
        installation: InstalledHardware,
        profile: PlatformProfile,
    ) -> Result<Self, HardwareError> {
        lab_model::validate_station(&configured).map_err(|e| HardwareError::Invalid(e.to_string()))?;
        if configured.cpu != installation.cpu {
            return Err(HardwareError::CpuMismatch {
                configured: configured.cpu,
                installed: installation.cpu,
            });
        }
        let mut station = Self {
            configured,
            installation,
            profile,
            faults: BTreeSet::new(),
            statuses: BTreeMap::new(),
            readings: BTreeMap::new(),
            outputs: BTreeMap::new(),
            last_run_outputs: BTreeMap::new(),
            buffer: DiagnosticBuffer::new(200),
            active: BTreeSet::new(),
        };
        station.refresh_status(WorldTime::ZERO);
        Ok(station)
    }

    pub fn configuration(&self) -> &StationConfig {
        &self.configured
    }
    pub fn profile(&self) -> &PlatformProfile {
        &self.profile
    }
    pub fn diagnostics(&self) -> &DiagnosticBuffer {
        &self.buffer
    }
    pub fn diagnostics_mut(&mut self) -> &mut DiagnosticBuffer {
        &mut self.buffer
    }

    /// Replace the configuration (hardware download). The CPU type must match.
    pub fn reconfigure(&mut self, configured: StationConfig, now: WorldTime) -> Result<(), HardwareError> {
        lab_model::validate_station(&configured).map_err(|e| HardwareError::Invalid(e.to_string()))?;
        if configured.cpu != self.installation.cpu {
            return Err(HardwareError::CpuMismatch {
                configured: configured.cpu,
                installed: self.installation.cpu,
            });
        }
        self.configured = configured;
        self.outputs.clear();
        self.last_run_outputs.clear();
        self.refresh_status(now);
        Ok(())
    }

    pub fn set_profile(&mut self, profile: PlatformProfile) {
        self.profile = profile;
    }

    pub fn apply_fault(&mut self, fault: HardwareFault, now: WorldTime) {
        self.faults.insert(fault);
        self.refresh_status(now);
    }
    pub fn clear_fault(&mut self, fault: &HardwareFault, now: WorldTime) -> bool {
        let removed = self.faults.remove(fault);
        self.refresh_status(now);
        removed
    }
    pub fn active_faults(&self) -> impl Iterator<Item = &HardwareFault> {
        self.faults.iter()
    }

    /// Physical modules after installation faults.
    fn effective(&self) -> BTreeMap<SlotRef, Installed> {
        let mut map: BTreeMap<SlotRef, Installed> = self
            .installation
            .modules
            .iter()
            .map(|m| {
                (
                    m.position,
                    Installed {
                        module: m.module,
                        powered: m.field_power,
                        wired_as: m.position,
                    },
                )
            })
            .collect();
        for fault in &self.faults {
            match fault {
                HardwareFault::ModuleMissing(p) => {
                    map.remove(p);
                }
                HardwareFault::WrongModule(p, module) => {
                    if let Some(m) = map.get_mut(p) {
                        m.module = *module;
                    } else {
                        map.insert(*p, Installed { module: *module, powered: true, wired_as: *p });
                    }
                }
                HardwareFault::WrongSlot(from, to) => {
                    if let Some(m) = map.remove(from) {
                        map.insert(*to, m);
                    }
                }
                HardwareFault::FieldPowerMissing(p) => {
                    if let Some(m) = map.get_mut(p) {
                        m.powered = false;
                    }
                }
                _ => {}
            }
        }
        map
    }

    fn status_of(&self, position: SlotRef, effective: &BTreeMap<SlotRef, Installed>) -> ModuleStatus {
        match (self.configured.module_at(position), effective.get(&position)) {
            (Some(_), None) => ModuleStatus::Missing,
            (Some(c), Some(i)) if c.module != i.module => ModuleStatus::Mismatch(i.module),
            (Some(_), Some(i)) if !i.powered => ModuleStatus::NoFieldPower,
            (Some(_), Some(_)) => ModuleStatus::Ok,
            (None, Some(_)) => ModuleStatus::Unconfigured,
            (None, None) => ModuleStatus::Missing,
        }
    }

    /// Recompute module status and log incoming/outgoing module diagnostics.
    pub fn refresh_status(&mut self, now: WorldTime) {
        let effective = self.effective();
        let positions: BTreeSet<SlotRef> = self
            .configured
            .modules
            .iter()
            .map(|m| m.position)
            .chain(effective.keys().copied())
            .collect();
        self.statuses = positions.iter().map(|p| (*p, self.status_of(*p, &effective))).collect();
        let mut current = BTreeSet::new();
        for (position, status) in &self.statuses {
            let source = DiagnosticSource::Module(*position);
            let kind = match status {
                ModuleStatus::Ok => continue,
                ModuleStatus::Missing => DiagnosticKind::ModuleMissing,
                ModuleStatus::Mismatch(installed) => DiagnosticKind::ModuleMismatch {
                    configured: self.configured.module_at(*position).map(|m| m.module),
                    installed: *installed,
                },
                ModuleStatus::NoFieldPower => DiagnosticKind::NoFieldPower,
                ModuleStatus::Unconfigured => DiagnosticKind::UnconfiguredModule,
            };
            current.insert((source, kind));
        }
        self.update_channel_diagnostics(&mut current);
        self.reconcile(current, now);
    }

    fn update_channel_diagnostics(&self, current: &mut BTreeSet<(DiagnosticSource, DiagnosticKind)>) {
        for (channel, reading) in &self.readings {
            let Some(config) = self.configured.channel_config(*channel) else { continue };
            if !config.diagnostics {
                continue;
            }
            let kind = match reading.quality {
                Quality::WireBreak => DiagnosticKind::WireBreak,
                Quality::Overflow => DiagnosticKind::Overflow,
                Quality::Underflow => DiagnosticKind::Underflow,
                _ => continue,
            };
            current.insert((DiagnosticSource::Channel(*channel), kind));
        }
    }

    fn reconcile(&mut self, current: BTreeSet<(DiagnosticSource, DiagnosticKind)>, now: WorldTime) {
        // Only hardware-owned sources are reconciled here; network sources are managed by the engine.
        let owned = |s: &DiagnosticSource| matches!(s, DiagnosticSource::Module(_) | DiagnosticSource::Channel(_));
        let gone: Vec<_> = self.active.iter().filter(|e| owned(&e.0) && !current.contains(*e)).cloned().collect();
        for (source, kind) in gone {
            self.active.remove(&(source, kind));
            self.buffer.push(DiagnosticEntry { time: now, source, kind, event: DiagnosticEvent::Outgoing });
        }
        for (source, kind) in current {
            if self.active.insert((source, kind)) {
                self.buffer.push(DiagnosticEntry { time: now, source, kind, event: DiagnosticEvent::Incoming });
            }
        }
    }

    /// Raise or clear an externally owned diagnostic (for example a network connection).
    pub fn set_external_diagnostic(&mut self, source: DiagnosticSource, kind: DiagnosticKind, active: bool, now: WorldTime) {
        let key = (source, kind);
        if active && self.active.insert(key) {
            self.buffer.push(DiagnosticEntry { time: now, source, kind, event: DiagnosticEvent::Incoming });
        } else if !active && self.active.remove(&key) {
            self.buffer.push(DiagnosticEntry { time: now, source, kind, event: DiagnosticEvent::Outgoing });
        }
    }

    pub fn active_diagnostics(&self) -> impl Iterator<Item = &(DiagnosticSource, DiagnosticKind)> {
        self.active.iter()
    }

    pub fn module_status(&self, position: SlotRef) -> ModuleStatus {
        self.statuses.get(&position).copied().unwrap_or(ModuleStatus::Missing)
    }

    /// Resolve which component port is physically connected to a terminal, after wiring faults.
    fn wire_at(&self, channel: ChannelRef, effective: &BTreeMap<SlotRef, Installed>) -> Option<(&str, &str)> {
        let installed = effective.get(&channel.slot_ref())?;
        let mut terminal = ChannelRef {
            rack: installed.wired_as.rack,
            slot: installed.wired_as.slot,
            channel: channel.channel,
        };
        for fault in &self.faults {
            if let HardwareFault::SwappedWires(a, b) = fault {
                if terminal == *a {
                    terminal = *b;
                } else if terminal == *b {
                    terminal = *a;
                }
            }
        }
        self.installation
            .wiring
            .iter()
            .find(|w| w.channel == terminal)
            .map(|w| (w.component.as_str(), w.port.as_str()))
    }

    fn open(&self, channel: ChannelRef) -> bool {
        self.faults.contains(&HardwareFault::OpenCircuit(channel))
    }
    fn shorted(&self, channel: ChannelRef) -> bool {
        self.faults.contains(&HardwareFault::ShortToSupply(channel))
    }

    /// Digital terminal voltage in millivolts (input "signal present" for either input style).
    fn digital_terminal(&self, channel: ChannelRef, style: DigitalInputStyle, scene: &SceneRuntime, effective: &BTreeMap<SlotRef, Installed>) -> i64 {
        if self.shorted(channel) {
            return 24_000;
        }
        if self.open(channel) {
            return 0;
        }
        let Some(installed) = effective.get(&channel.slot_ref()) else { return 0 };
        if !installed.powered {
            return 0;
        }
        let Some((component, port)) = self.wire_at(channel, effective) else { return 0 };
        let Ok(index) = scene.index_of_str(component) else { return 0 };
        let Ok(field) = scene.digital_output(index, port) else { return 0 };
        let compatible = match (style, field.style) {
            (_, OutputStyle::DryContact) => true,
            (DigitalInputStyle::Sinking, OutputStyle::Pnp) => true,
            (DigitalInputStyle::Sourcing, OutputStyle::Npn) => true,
            _ => false,
        };
        if field.conducting && compatible { 24_000 } else { 0 }
    }

    fn analog_field(&self, channel: ChannelRef, scene: &SceneRuntime, effective: &BTreeMap<SlotRef, Installed>) -> Option<AnalogField> {
        if self.open(channel) {
            return None;
        }
        let (component, port) = self.wire_at(channel, effective)?;
        let index = scene.index_of_str(component).ok()?;
        scene.analog_output(index, port).ok()
    }

    /// Sample every configured input channel from the physical process.
    pub fn sample_inputs(&mut self, scene: &SceneRuntime, now: WorldTime) -> &BTreeMap<ChannelRef, ChannelReading> {
        let effective = self.effective();
        let mut readings = BTreeMap::new();
        for module in &self.configured.modules {
            let status = self.status_of(module.position, &effective);
            for channel_index in 0..module.module.channel_count() {
                let channel = ChannelRef { rack: module.position.rack, slot: module.position.slot, channel: channel_index };
                let config = self.configured.channel_config(channel);
                let mode = config.map_or(default_mode(module.module), |c| c.mode);
                let reading = match module.module.class() {
                    ModuleClass::DigitalInput { style, .. } => {
                        if mode == ChannelMode::Disabled {
                            ChannelReading { value: ChannelValue::Bool(false), quality: Quality::ChannelDisabled }
                        } else {
                            match status {
                                ModuleStatus::Ok => ChannelReading {
                                    value: ChannelValue::Bool(self.digital_terminal(channel, style, scene, &effective) > 15_000),
                                    quality: Quality::Good,
                                },
                                ModuleStatus::NoFieldPower => ChannelReading { value: ChannelValue::Bool(false), quality: Quality::NoFieldPower },
                                _ => ChannelReading { value: ChannelValue::Bool(false), quality: Quality::ModuleFault },
                            }
                        }
                    }
                    ModuleClass::AnalogInput { .. } => {
                        let fallback = match self.profile.analog {
                            lab_model::profile::AnalogRepresentation::EngineeringUnits => ChannelValue::Real(0.0),
                            _ => ChannelValue::Int(0),
                        };
                        if mode == ChannelMode::Disabled {
                            ChannelReading { value: fallback, quality: Quality::ChannelDisabled }
                        } else if status != ModuleStatus::Ok && status != ModuleStatus::NoFieldPower {
                            ChannelReading { value: fallback, quality: Quality::ModuleFault }
                        } else {
                            let field = self.analog_field(channel, scene, &effective);
                            let diagnostics = config.is_some_and(|c| c.diagnostics);
                            let shorted = self.shorted(channel);
                            let field = if shorted {
                                Some(AnalogField { unit: process_simulation::AnalogUnit::Millivolt, value: 24_000, signal: lab_model::scene::AnalogSignal::Voltage0To10V })
                            } else {
                                field
                            };
                            match analog::convert(field, mode, diagnostics, self.profile.analog) {
                                Some(r) => ChannelReading {
                                    value: match r.raw {
                                        AnalogRaw::Int(v) => ChannelValue::Int(v),
                                        AnalogRaw::Real(v) => ChannelValue::Real(v),
                                    },
                                    quality: match r.condition {
                                        AnalogCondition::Normal => Quality::Good,
                                        AnalogCondition::Overrange => Quality::Overrange,
                                        AnalogCondition::Underrange => Quality::Underrange,
                                        AnalogCondition::Overflow => Quality::Overflow,
                                        AnalogCondition::Underflow => Quality::Underflow,
                                        AnalogCondition::WireBreak => Quality::WireBreak,
                                    },
                                },
                                None => ChannelReading { value: fallback, quality: Quality::ChannelDisabled },
                            }
                        }
                    }
                    _ => continue,
                };
                readings.insert(channel, reading);
            }
        }
        self.readings = readings;
        let mut current: BTreeSet<_> = self
            .active
            .iter()
            .filter(|(s, _)| matches!(s, DiagnosticSource::Module(_)))
            .cloned()
            .collect();
        self.update_channel_diagnostics(&mut current);
        self.reconcile(current, now);
        &self.readings
    }

    pub fn reading(&self, channel: ChannelRef) -> Option<ChannelReading> {
        self.readings.get(&channel).copied()
    }

    /// Apply output channel commands (CPU in RUN) and drive wired actuators.
    pub fn drive_outputs(&mut self, commands: &BTreeMap<ChannelRef, bool>, cpu_running: bool, scene: &mut SceneRuntime) -> Result<(), HardwareError> {
        let effective = self.effective();
        let mut outputs = BTreeMap::new();
        for module in &self.configured.modules {
            if !matches!(module.module.class(), ModuleClass::DigitalOutput { .. }) {
                continue;
            }
            for index in 0..module.module.channel_count() {
                let channel = ChannelRef { rack: module.position.rack, slot: module.position.slot, channel: index };
                let config = self.configured.channel_config(channel);
                let enabled = config.is_none_or(|c| c.mode != ChannelMode::Disabled);
                let requested = commands.get(&channel).copied().unwrap_or(false);
                let value = if cpu_running {
                    requested
                } else {
                    match config.map(|c| c.stop_behavior).unwrap_or_default() {
                        OutputStopBehavior::Off => false,
                        OutputStopBehavior::HoldLast => self.last_run_outputs.get(&channel).copied().unwrap_or(false),
                    }
                };
                outputs.insert(channel, enabled && value);
            }
        }
        if cpu_running {
            self.last_run_outputs = outputs.clone();
        }
        // De-energize every wired sink first, then energize the driven ones.
        let mut energize: BTreeMap<(String, String), bool> = BTreeMap::new();
        for wire in &self.installation.wiring {
            let Ok(index) = scene.index_of_str(wire.component.as_str()) else { continue };
            let is_sink = scene
                .component_kind(index)
                .is_some()
                && !wire.port.is_empty()
                && matches!(wire.port.as_str(), "coil" | "solenoid" | "motor");
            if is_sink {
                energize.insert((wire.component.to_string(), wire.port.clone()), false);
            }
        }
        for (channel, on) in &outputs {
            let Some(installed) = effective.get(&channel.slot_ref()) else { continue };
            let module_ok = self.status_of(channel.slot_ref(), &effective) == ModuleStatus::Ok;
            let live = *on && module_ok && installed.powered && !self.open(*channel);
            if let Some((component, port)) = self.wire_at(*channel, &effective)
                && let Some(slot) = energize.get_mut(&(component.to_string(), port.to_string()))
            {
                *slot = *slot || live;
            }
        }
        for ((component, port), on) in energize {
            let index = scene.index_of_str(&component)?;
            scene.set_coil(index, &port, on)?;
        }
        self.outputs = outputs;
        Ok(())
    }

    pub fn output(&self, channel: ChannelRef) -> Option<bool> {
        self.outputs.get(&channel).copied()
    }

    /// What a multimeter shows at a terminal right now.
    pub fn measure(&self, channel: ChannelRef, scene: &SceneRuntime) -> Measurement {
        let effective = self.effective();
        let Some(installed) = effective.get(&channel.slot_ref()) else { return Measurement::NoModule };
        match installed.module.class() {
            ModuleClass::DigitalInput { style, .. } => Measurement::Millivolts(self.digital_terminal(channel, style, scene, &effective)),
            ModuleClass::DigitalOutput { .. } => {
                let module_ok = self.status_of(channel.slot_ref(), &effective) == ModuleStatus::Ok;
                let on = self.outputs.get(&channel).copied().unwrap_or(false) && module_ok && installed.powered;
                Measurement::Millivolts(if on { 24_000 } else { 0 })
            }
            ModuleClass::AnalogInput { .. } => match self.analog_field(channel, scene, &effective) {
                Some(f) => match f.unit {
                    process_simulation::AnalogUnit::Microamp => Measurement::Microamps(f.value),
                    process_simulation::AnalogUnit::Millivolt => Measurement::Millivolts(f.value),
                },
                None => Measurement::Microamps(0),
            },
            _ => Measurement::NoModule,
        }
    }

    pub fn tag_for(&self, channel: ChannelRef) -> Option<&str> {
        self.configured.io_map.iter().find(|m| m.channel == channel).map(|m| m.tag.as_str())
    }

    pub fn module_views(&self) -> Vec<ModuleView> {
        let effective = self.effective();
        let positions: BTreeSet<SlotRef> = self
            .configured
            .modules
            .iter()
            .map(|m| m.position)
            .chain(effective.keys().copied())
            .collect();
        positions
            .into_iter()
            .map(|position| {
                let configured = self.configured.module_at(position);
                let installed = effective.get(&position).map(|i| i.module);
                let status = self.statuses.get(&position).copied().unwrap_or(ModuleStatus::Missing);
                let module_type = configured.map(|c| c.module).or(installed);
                let channel_faults = self.active.iter().any(|(s, _)| matches!(s, DiagnosticSource::Channel(c) if c.slot_ref() == position));
                let status_led = match status {
                    ModuleStatus::Ok if channel_faults => LedState::RedFlashing,
                    ModuleStatus::Ok => LedState::Green,
                    ModuleStatus::Missing => LedState::Off,
                    ModuleStatus::Mismatch(_) | ModuleStatus::NoFieldPower => LedState::RedFlashing,
                    ModuleStatus::Unconfigured => LedState::GreenFlashing,
                };
                let channels = module_type
                    .map(|m| {
                        (0..m.channel_count())
                            .map(|index| {
                                let channel = ChannelRef { rack: position.rack, slot: position.slot, channel: index };
                                let reading = self.readings.get(&channel).copied();
                                let output = self.outputs.get(&channel).copied();
                                let led = status != ModuleStatus::Missing
                                    && match m.class() {
                                        ModuleClass::DigitalInput { .. } => matches!(reading, Some(ChannelReading { value: ChannelValue::Bool(true), .. })),
                                        ModuleClass::DigitalOutput { .. } => output == Some(true) && status == ModuleStatus::Ok,
                                        _ => false,
                                    };
                                ChannelView {
                                    channel: index,
                                    mode: self.configured.channel_config(channel).map_or(default_mode(m), |c| c.mode),
                                    tag: self.tag_for(channel).map(str::to_owned),
                                    address: address::format(self.profile.addressing, m, configured.map_or(0, |c| c.start_address), channel, self.profile.product_name(m)),
                                    reading,
                                    output,
                                    led,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                ModuleView {
                    position,
                    configured: configured.map(|c| c.module),
                    installed,
                    status,
                    product: module_type.and_then(|m| self.profile.product_name(m).map(str::to_owned)),
                    status_led,
                    channels,
                }
            })
            .collect()
    }
}

pub fn default_mode(module: ModuleType) -> ChannelMode {
    match module.class() {
        ModuleClass::AnalogInput { .. } | ModuleClass::AnalogOutput { .. } => ChannelMode::Current4To20mA,
        _ => ChannelMode::Digital,
    }
}
