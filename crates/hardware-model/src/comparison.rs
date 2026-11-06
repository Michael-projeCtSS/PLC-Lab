//! Configured versus actual hardware: typed findings derived by comparing the project's
//! station configuration with the installation as it is (including installation faults),
//! and the configuration with the drawing (the documented field wiring and devices).
//!
//! Findings are evidence: what the CPU or an engineer can establish by comparing the
//! project, the drawing and what is detected. They never name an injected fault.

use crate::{StationRuntime, address, default_mode};
use lab_model::{
    hardware::{ChannelMode, ChannelRef, ModuleClass, SlotRef},
    message,
    message::{Arg, Message, term},
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
    pub expected: Option<Message>,
    /// What is detected or configured instead.
    pub detected: Option<Message>,
    pub message: Message,
}

/// "rack 0 slot 2" as a message, so the interface words the position itself.
pub fn slot_message(slot: SlotRef) -> Message {
    message!(
        "hw.slot",
        "rack {rack} slot {slot}",
        rack = slot.rack,
        slot = slot.slot
    )
}

pub fn signal_term(s: AnalogSignal) -> Arg {
    match s {
        AnalogSignal::Current4To20mA => term("signal.4-20mA", "4-20 mA"),
        AnalogSignal::Current0To20mA => term("signal.0-20mA", "0-20 mA"),
        AnalogSignal::Voltage0To10V => term("signal.0-10V", "0-10 V"),
    }
}

pub fn mode_term(m: ChannelMode) -> Arg {
    match m {
        ChannelMode::Disabled => term("mode.disabled", "disabled"),
        ChannelMode::Digital => term("mode.digital", "digital"),
        ChannelMode::Current4To20mA => term("signal.4-20mA", "4-20 mA"),
        ChannelMode::Current0To20mA => term("signal.0-20mA", "0-20 mA"),
        ChannelMode::Voltage0To10V => term("signal.0-10V", "0-10 V"),
    }
}

/// A single value as a finding's expected or detected side.
fn value(arg: Arg) -> Message {
    message!("hw.value", "{value}", value = arg)
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
            let name = module.module.name_term();
            let id = module.id.to_string();
            let here = slot_message(module.position);
            let finding =
                |category, severity, detected: Option<Message>, message: Message| HardwareFinding {
                    category,
                    severity,
                    slot: Some(module.position),
                    channel: None,
                    module_id: Some(module.id.to_string()),
                    expected: Some(value(module.module.name_term())),
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
                            Some(message!(
                                "hw.detected.at",
                                "{name} at {slot}",
                                name = name.clone(),
                                slot = slot_message(at)
                            )),
                            message!(
                                "hw.wrongSlot",
                                "{id} ({name}) is configured at {configured}, which is empty; a {name} is installed at {installed}, which the project does not configure",
                                id = id.clone(),
                                name = name.clone(),
                                configured = here.clone(),
                                installed = slot_message(at)
                            ),
                        ));
                    } else {
                        out.push(finding(
                            FindingCategory::ModuleMissing,
                            FindingSeverity::Error,
                            None,
                            message!(
                                "hw.missing",
                                "{id} ({name}) is configured at {slot}, no module answers there",
                                id = id.clone(),
                                name = name.clone(),
                                slot = here.clone()
                            ),
                        ));
                    }
                }
                Some(i) if !i.electronics => out.push(finding(
                    FindingCategory::ModulePowerMissing,
                    FindingSeverity::Error,
                    Some(message!("hw.detected.dark", "installed, status LEDs dark")),
                    message!(
                        "hw.modulePower",
                        "{id} at {slot}: a module is installed but does not answer; its status LEDs are dark",
                        id = id.clone(),
                        slot = here.clone()
                    ),
                )),
                Some(i) if i.module != module.module => out.push(finding(
                    FindingCategory::WrongModuleType,
                    FindingSeverity::Error,
                    Some(value(i.module.name_term())),
                    message!(
                        "hw.wrongType",
                        "{id} at {slot}: configured {name}, detected {detected}",
                        id = id.clone(),
                        slot = here.clone(),
                        name = name.clone(),
                        detected = i.module.name_term()
                    ),
                )),
                Some(i) if !i.powered && has_field_side(module.module.class()) => out.push(finding(
                    FindingCategory::FieldPowerMissing,
                    FindingSeverity::Error,
                    Some(message!("hw.detected.noField", "field (load) supply missing")),
                    message!(
                        "hw.fieldPower",
                        "{id} at {slot}: the module answers, but its 24 V field (load) supply is missing",
                        id = id.clone(),
                        slot = here.clone()
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
                detected: Some(value(installed.module.name_term())),
                message: message!(
                    "hw.unexpected",
                    "{name} is installed at {slot}, which the project does not configure",
                    name = installed.module.name_term(),
                    slot = slot_message(*slot)
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
            let base = |category, expected: Arg, detected: Arg, message: Message| HardwareFinding {
                category,
                severity: FindingSeverity::Error,
                slot: Some(module.position),
                channel: Some(wire.channel),
                module_id: Some(module.id.to_string()),
                expected: Some(value(expected)),
                detected: Some(value(detected)),
                message,
            };
            if mode != expected {
                out.push(base(
                    FindingCategory::ChannelModeMismatch,
                    signal_term(field.signal),
                    mode_term(mode),
                    message!(
                        "hw.channelMode",
                        "{channel}: configured {mode}, the drawing shows {label} with a {signal} signal",
                        channel = channel_id.clone(),
                        mode = mode_term(mode),
                        label = label,
                        signal = signal_term(field.signal)
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
                    (
                        term("loop.supplied2wire", "loop supplied by the input (2-wire)"),
                        term("loop.passive", "passive input"),
                    )
                } else {
                    (
                        term("loop.passive4wire", "passive input (4-wire)"),
                        term("loop.supplied", "loop supplied by the input"),
                    )
                };
                out.push(base(
                    FindingCategory::LoopSupplyMismatch,
                    wanted.clone(),
                    configured.clone(),
                    message!(
                        "hw.loopSupply",
                        "{channel}: {label} needs a {wanted}, the channel is configured as {configured}",
                        channel = channel_id.clone(),
                        label = label,
                        wanted = wanted,
                        configured = configured
                    ),
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
                        expected: Some(message!(
                            "hw.expected.freeAddresses",
                            "addresses not used by {module}",
                            module = a.id.to_string()
                        )),
                        detected: Some(message!(
                            "hw.detected.bytes",
                            "bytes {first}..{last}",
                            first = b_range.start,
                            last = b_range.end - 1
                        )),
                        message: message!(
                            "hw.addressOverlap",
                            "{a} and {b} both use {direction} bytes {first}..{last}",
                            a = a.id.to_string(),
                            b = b.id.to_string(),
                            direction = if *a_input {
                                term("io.input", "input")
                            } else {
                                term("io.output", "output")
                            },
                            first = a_range.start.max(b_range.start),
                            last = a_range.end.min(b_range.end) - 1
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
            let tag = mapping.tag.to_string();
            let mut push = |severity, message: Message| {
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
                    detected: Some(value(Arg::Data(mapping.tag.to_string()))),
                    message,
                });
            };
            match self.configured.module_at(mapping.channel.slot_ref()) {
                None => push(
                    FindingSeverity::Error,
                    message!(
                        "hw.binding.noModule",
                        "{tag} is mapped to {slot}, where no module is configured",
                        tag = tag.clone(),
                        slot = slot_message(mapping.channel.slot_ref())
                    ),
                ),
                Some(m) if mapping.channel.channel >= m.module.channel_count() => push(
                    FindingSeverity::Error,
                    message!(
                        "hw.binding.noChannel",
                        "{tag} is mapped to channel {channel} of {module}, which has only {count} channels",
                        tag = tag.clone(),
                        channel = mapping.channel.channel,
                        module = m.id.to_string(),
                        count = m.module.channel_count()
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
                        message!(
                            "hw.binding.disabled",
                            "{tag} is mapped to {channel}, which is disabled",
                            tag = tag.clone(),
                            channel = m.channel_id(mapping.channel.channel)
                        ),
                    )
                }
                Some(_) => {}
            }
            if !channels.insert(mapping.channel) {
                push(
                    FindingSeverity::Error,
                    message!(
                        "hw.binding.shared",
                        "{tag} shares its channel with another tag",
                        tag = tag.clone()
                    ),
                );
            }
            if !tags.insert(mapping.tag.as_str()) {
                push(
                    FindingSeverity::Error,
                    message!(
                        "hw.binding.twice",
                        "{tag} is mapped to more than one channel",
                        tag = tag.clone()
                    ),
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
