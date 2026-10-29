//! Configured versus actual hardware: typed findings derived by comparing the project's
//! station configuration with the installation as it is (including installation faults),
//! and the configuration with the drawing (the documented field wiring and devices).
//!
//! Findings are evidence: what the CPU or an engineer can establish by comparing the
//! project, the drawing and what is detected. They never name an injected fault.

use crate::{StationRuntime, address, default_mode};
use lab_model::{
    hardware::{ChannelMode, ChannelRef, ModuleClass, SlotRef},
    profile::AddressingScheme,
    scene::AnalogSignal,
};
use process_simulation::SceneRuntime;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingCategory {
    ModuleMissing,
    UnexpectedModule,
    WrongModuleType,
    /// A configured module is missing and a module of the same type sits in a slot the
    /// project does not configure.
    WrongSlot,
    ModulePowerMissing,
    FieldPowerMissing,
    /// An analog channel's mode does not match the signal of the device on the drawing.
    ChannelModeMismatch,
    /// A current input's loop supply does not match the transmitter on the drawing.
    LoopSupplyMismatch,
    /// Two modules use the same process image addresses.
    AddressOverlap,
    /// An I/O mapping points to no usable channel, or a channel or tag is mapped twice.
    SignalBindingMismatch,
}

impl FindingCategory {
    pub fn code(self) -> &'static str {
        match self {
            Self::ModuleMissing => "MODULE_MISSING",
            Self::UnexpectedModule => "UNEXPECTED_MODULE",
            Self::WrongModuleType => "WRONG_MODULE_TYPE",
            Self::WrongSlot => "WRONG_SLOT",
            Self::ModulePowerMissing => "MODULE_POWER_MISSING",
            Self::FieldPowerMissing => "FIELD_POWER_MISSING",
            Self::ChannelModeMismatch => "CHANNEL_MODE_MISMATCH",
            Self::LoopSupplyMismatch => "LOOP_SUPPLY_MISMATCH",
            Self::AddressOverlap => "ADDRESS_OVERLAP",
            Self::SignalBindingMismatch => "SIGNAL_BINDING_MISMATCH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareFinding {
    pub category: FindingCategory,
    pub severity: FindingSeverity,
    pub slot: Option<SlotRef>,
    pub channel: Option<ChannelRef>,
    /// The configured module concerned, by its stable id.
    pub module_id: Option<String>,
    /// What the project (or the drawing) expects.
    pub expected: Option<String>,
    /// What is detected or configured instead.
    pub detected: Option<String>,
    pub message: String,
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

fn mode_text(m: ChannelMode) -> &'static str {
    match m {
        ChannelMode::Disabled => "disabled",
        ChannelMode::Digital => "digital",
        ChannelMode::Current4To20mA => "4-20 mA",
        ChannelMode::Current0To20mA => "0-20 mA",
        ChannelMode::Voltage0To10V => "0-10 V",
    }
}

impl StationRuntime {
    /// Every difference between the project, the installation and the drawing.
    pub fn compare(&self, scene: &SceneRuntime) -> Vec<HardwareFinding> {
        let mut out = Vec::new();
        self.compare_modules(&mut out);
        self.compare_channels(scene, &mut out);
        self.compare_addresses(&mut out);
        self.compare_bindings(&mut out);
        out
    }

    fn compare_modules(&self, out: &mut Vec<HardwareFinding>) {
        let effective = self.effective();
        let configured_slots: BTreeSet<SlotRef> =
            self.configured.modules.iter().map(|m| m.position).collect();
        // Unconfigured installed modules, by type, for the wrong-slot comparison.
        let mut spare: BTreeMap<_, Vec<SlotRef>> = BTreeMap::new();
        for (slot, installed) in &effective {
            if !configured_slots.contains(slot) && installed.electronics {
                spare.entry(installed.module).or_default().push(*slot);
            }
        }
        let mut placed = BTreeSet::new();
        for module in &self.configured.modules {
            let name = module.module.generic_name();
            let finding =
                |category, severity, detected: Option<String>, message: String| HardwareFinding {
                    category,
                    severity,
                    slot: Some(module.position),
                    channel: None,
                    module_id: Some(module.id.to_string()),
                    expected: Some(name.to_owned()),
                    detected,
                    message,
                };
            match effective.get(&module.position) {
                None => {
                    let elsewhere = spare
                        .get(&module.module)
                        .and_then(|slots| slots.iter().find(|s| !placed.contains(*s)).copied());
                    if let Some(at) = elsewhere {
                        placed.insert(at);
                        out.push(finding(
                            FindingCategory::WrongSlot,
                            FindingSeverity::Error,
                            Some(format!("{name} at {}", slot_text(at))),
                            format!(
                                "{} ({name}) is configured at {}, which is empty; a {name} is installed at {}, which the project does not configure",
                                module.id,
                                slot_text(module.position),
                                slot_text(at)
                            ),
                        ));
                    } else {
                        out.push(finding(
                            FindingCategory::ModuleMissing,
                            FindingSeverity::Error,
                            None,
                            format!(
                                "{} ({name}) is configured at {}, no module answers there",
                                module.id,
                                slot_text(module.position)
                            ),
                        ));
                    }
                }
                Some(i) if !i.electronics => out.push(finding(
                    FindingCategory::ModulePowerMissing,
                    FindingSeverity::Error,
                    Some("installed, status LEDs dark".into()),
                    format!(
                        "{} at {}: a module is installed but does not answer; its status LEDs are dark",
                        module.id,
                        slot_text(module.position)
                    ),
                )),
                Some(i) if i.module != module.module => out.push(finding(
                    FindingCategory::WrongModuleType,
                    FindingSeverity::Error,
                    Some(i.module.generic_name().to_owned()),
                    format!(
                        "{} at {}: configured {name}, detected {}",
                        module.id,
                        slot_text(module.position),
                        i.module.generic_name()
                    ),
                )),
                Some(i) if !i.powered && has_field_side(module.module.class()) => out.push(finding(
                    FindingCategory::FieldPowerMissing,
                    FindingSeverity::Error,
                    Some("field (load) supply missing".into()),
                    format!(
                        "{} at {}: the module answers, but its 24 V field (load) supply is missing",
                        module.id,
                        slot_text(module.position)
                    ),
                )),
                Some(_) => {}
            }
        }
        for (slot, installed) in &effective {
            if configured_slots.contains(slot) || placed.contains(slot) {
                continue;
            }
            out.push(HardwareFinding {
                category: FindingCategory::UnexpectedModule,
                severity: FindingSeverity::Warning,
                slot: Some(*slot),
                channel: None,
                module_id: None,
                expected: None,
                detected: Some(installed.module.generic_name().to_owned()),
                message: format!(
                    "{} is installed at {}, which the project does not configure",
                    installed.module.generic_name(),
                    slot_text(*slot)
                ),
            });
        }
    }

    /// Analog channel modes and loop supply against the devices on the drawing.
    fn compare_channels(&self, scene: &SceneRuntime, out: &mut Vec<HardwareFinding>) {
        for wire in &self.installation.wiring {
            let Some(module) = self.configured.module_at(wire.channel.slot_ref()) else {
                continue;
            };
            if !matches!(module.module.class(), ModuleClass::AnalogInput { .. }) {
                continue;
            }
            let Ok(index) = scene.index_of_str(wire.component.as_str()) else {
                continue;
            };
            let Ok(field) = scene.analog_output(index, &wire.port) else {
                continue;
            };
            let config = self.configured.channel_config(wire.channel);
            let mode = config.map_or(default_mode(module.module), |c| c.mode);
            let expected = match field.signal {
                AnalogSignal::Current4To20mA => ChannelMode::Current4To20mA,
                AnalogSignal::Current0To20mA => ChannelMode::Current0To20mA,
                AnalogSignal::Voltage0To10V => ChannelMode::Voltage0To10V,
            };
            let label = scene
                .component_label(index)
                .unwrap_or(wire.component.as_str());
            let channel_id = module.channel_id(wire.channel.channel);
            let base =
                |category, expected: String, detected: String, message: String| HardwareFinding {
                    category,
                    severity: FindingSeverity::Error,
                    slot: Some(module.position),
                    channel: Some(wire.channel),
                    module_id: Some(module.id.to_string()),
                    expected: Some(expected),
                    detected: Some(detected),
                    message,
                };
            if mode != expected {
                out.push(base(
                    FindingCategory::ChannelModeMismatch,
                    signal_text(field.signal).into(),
                    mode_text(mode).into(),
                    format!(
                        "{channel_id}: configured {}, the drawing shows {label} with a {} signal",
                        mode_text(mode),
                        signal_text(field.signal)
                    ),
                ));
                continue;
            }
            let supplied = config.is_none_or(|c| c.loop_supply);
            let current = matches!(
                mode,
                ChannelMode::Current4To20mA | ChannelMode::Current0To20mA
            );
            if current && field.loop_powered != supplied {
                let (wanted, configured) = if field.loop_powered {
                    ("loop supplied by the input (2-wire)", "passive input")
                } else {
                    ("passive input (4-wire)", "loop supplied by the input")
                };
                out.push(base(
                    FindingCategory::LoopSupplyMismatch,
                    wanted.into(),
                    configured.into(),
                    format!("{channel_id}: {label} needs a {wanted}, the channel is configured as {configured}"),
                ));
            }
        }
    }

    /// Overlapping process image addresses (byte-oriented addressing schemes only).
    fn compare_addresses(&self, out: &mut Vec<HardwareFinding>) {
        if !matches!(
            self.profile.addressing,
            AddressingScheme::IecDirect | AddressingScheme::ByteBit
        ) {
            return;
        }
        let ranges: Vec<_> = self
            .configured
            .modules
            .iter()
            .filter_map(|m| address::image_range(m.module, m.start_address).map(|r| (m, r)))
            .collect();
        for (i, (a, (a_input, a_range))) in ranges.iter().enumerate() {
            for (b, (b_input, b_range)) in &ranges[i + 1..] {
                if a_input == b_input && a_range.start < b_range.end && b_range.start < a_range.end
                {
                    out.push(HardwareFinding {
                        category: FindingCategory::AddressOverlap,
                        severity: FindingSeverity::Error,
                        slot: Some(b.position),
                        channel: None,
                        module_id: Some(b.id.to_string()),
                        expected: Some(format!("addresses not used by {}", a.id)),
                        detected: Some(format!("bytes {}..{}", b_range.start, b_range.end - 1)),
                        message: format!(
                            "{} and {} both use {} bytes {}..{}",
                            a.id,
                            b.id,
                            if *a_input { "input" } else { "output" },
                            a_range.start.max(b_range.start),
                            a_range.end.min(b_range.end) - 1
                        ),
                    });
                }
            }
        }
    }

    /// I/O mappings that point to no usable channel, and duplicates.
    fn compare_bindings(&self, out: &mut Vec<HardwareFinding>) {
        let mut channels = BTreeSet::new();
        let mut tags = BTreeSet::new();
        for mapping in &self.configured.io_map {
            let mut push = |severity, message: String| {
                out.push(HardwareFinding {
                    category: FindingCategory::SignalBindingMismatch,
                    severity,
                    slot: Some(mapping.channel.slot_ref()),
                    channel: Some(mapping.channel),
                    module_id: self
                        .configured
                        .module_at(mapping.channel.slot_ref())
                        .map(|m| m.id.to_string()),
                    expected: None,
                    detected: Some(mapping.tag.to_string()),
                    message,
                });
            };
            match self.configured.module_at(mapping.channel.slot_ref()) {
                None => push(
                    FindingSeverity::Error,
                    format!(
                        "{} is mapped to {}, where no module is configured",
                        mapping.tag,
                        slot_text(mapping.channel.slot_ref())
                    ),
                ),
                Some(m) if mapping.channel.channel >= m.module.channel_count() => push(
                    FindingSeverity::Error,
                    format!(
                        "{} is mapped to channel {} of {}, which has only {} channels",
                        mapping.tag,
                        mapping.channel.channel,
                        m.id,
                        m.module.channel_count()
                    ),
                ),
                Some(m)
                    if self
                        .configured
                        .channel_config(mapping.channel)
                        .is_some_and(|c| c.mode == ChannelMode::Disabled) =>
                {
                    push(
                        FindingSeverity::Warning,
                        format!(
                            "{} is mapped to {}, which is disabled",
                            mapping.tag,
                            m.channel_id(mapping.channel.channel)
                        ),
                    )
                }
                Some(_) => {}
            }
            if !channels.insert(mapping.channel) {
                push(
                    FindingSeverity::Error,
                    format!("{} shares its channel with another tag", mapping.tag),
                );
            }
            if !tags.insert(mapping.tag.as_str()) {
                push(
                    FindingSeverity::Error,
                    format!("{} is mapped to more than one channel", mapping.tag),
                );
            }
        }
    }
}

/// Modules whose channels need a field (load / sensor) supply.
fn has_field_side(class: ModuleClass) -> bool {
    matches!(
        class,
        ModuleClass::DigitalInput { .. }
            | ModuleClass::DigitalOutput { .. }
            | ModuleClass::AnalogInput { .. }
            | ModuleClass::AnalogOutput { .. }
    )
}
