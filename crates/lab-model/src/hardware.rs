//! Hardware data: the generic module catalog, physically installed hardware with field
//! wiring, and the engineer's configured station (what the project says is installed).
//!
//! Vendor product names are profile data. Behavior is keyed only by generic module type.

use crate::{
    ids::{Identifier, TagName},
    network::StationNetworkConfig,
};
use serde::{Deserialize, Serialize};

/// Generic module type identifiers understood by the hardware model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ModuleType {
    #[serde(rename = "cpu.compact")]
    CpuCompact,
    #[serde(rename = "cpu.modular")]
    CpuModular,
    #[serde(rename = "ps.24vdc")]
    PowerSupply24Vdc,
    #[serde(rename = "di8.24vdc.sink")]
    Di8Sinking,
    #[serde(rename = "di16.24vdc.sink")]
    Di16Sinking,
    #[serde(rename = "di8.24vdc.source")]
    Di8Sourcing,
    #[serde(rename = "do8.24vdc")]
    Do8,
    #[serde(rename = "do16.24vdc")]
    Do16,
    #[serde(rename = "ai4.ui")]
    Ai4,
    #[serde(rename = "ai8.ui")]
    Ai8,
    #[serde(rename = "ao2.ui")]
    Ao2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitalInputStyle {
    /// Current flows into the input: use with sourcing (PNP) devices or contacts from L+.
    Sinking,
    /// Current flows out of the input: use with sinking (NPN) devices or contacts to M.
    Sourcing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleClass {
    Cpu,
    PowerSupply,
    DigitalInput { channels: u8, style: DigitalInputStyle },
    DigitalOutput { channels: u8 },
    AnalogInput { channels: u8 },
    AnalogOutput { channels: u8 },
}

impl ModuleType {
    pub const ALL: [ModuleType; 11] = [
        Self::CpuCompact,
        Self::CpuModular,
        Self::PowerSupply24Vdc,
        Self::Di8Sinking,
        Self::Di16Sinking,
        Self::Di8Sourcing,
        Self::Do8,
        Self::Do16,
        Self::Ai4,
        Self::Ai8,
        Self::Ao2,
    ];
    pub fn class(self) -> ModuleClass {
        use DigitalInputStyle::*;
        match self {
            Self::CpuCompact | Self::CpuModular => ModuleClass::Cpu,
            Self::PowerSupply24Vdc => ModuleClass::PowerSupply,
            Self::Di8Sinking => ModuleClass::DigitalInput {
                channels: 8,
                style: Sinking,
            },
            Self::Di16Sinking => ModuleClass::DigitalInput {
                channels: 16,
                style: Sinking,
            },
            Self::Di8Sourcing => ModuleClass::DigitalInput {
                channels: 8,
                style: Sourcing,
            },
            Self::Do8 => ModuleClass::DigitalOutput { channels: 8 },
            Self::Do16 => ModuleClass::DigitalOutput { channels: 16 },
            Self::Ai4 => ModuleClass::AnalogInput { channels: 4 },
            Self::Ai8 => ModuleClass::AnalogInput { channels: 8 },
            Self::Ao2 => ModuleClass::AnalogOutput { channels: 2 },
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::CpuCompact => "cpu.compact",
            Self::CpuModular => "cpu.modular",
            Self::PowerSupply24Vdc => "ps.24vdc",
            Self::Di8Sinking => "di8.24vdc.sink",
            Self::Di16Sinking => "di16.24vdc.sink",
            Self::Di8Sourcing => "di8.24vdc.source",
            Self::Do8 => "do8.24vdc",
            Self::Do16 => "do16.24vdc",
            Self::Ai4 => "ai4.ui",
            Self::Ai8 => "ai8.ui",
            Self::Ao2 => "ao2.ui",
        }
    }
    pub fn generic_name(self) -> &'static str {
        match self {
            Self::CpuCompact => "Compact CPU",
            Self::CpuModular => "Modular CPU",
            Self::PowerSupply24Vdc => "Power supply 24 V DC",
            Self::Di8Sinking => "Digital input 8 x 24 V DC (sinking)",
            Self::Di16Sinking => "Digital input 16 x 24 V DC (sinking)",
            Self::Di8Sourcing => "Digital input 8 x 24 V DC (sourcing)",
            Self::Do8 => "Digital output 8 x 24 V DC / 0.5 A",
            Self::Do16 => "Digital output 16 x 24 V DC / 0.5 A",
            Self::Ai4 => "Analog input 4 x U/I",
            Self::Ai8 => "Analog input 8 x U/I",
            Self::Ao2 => "Analog output 2 x U/I",
        }
    }
    pub fn channel_count(self) -> u8 {
        match self.class() {
            ModuleClass::DigitalInput { channels, .. }
            | ModuleClass::DigitalOutput { channels }
            | ModuleClass::AnalogInput { channels }
            | ModuleClass::AnalogOutput { channels } => channels,
            ModuleClass::Cpu | ModuleClass::PowerSupply => 0,
        }
    }
    pub fn is_io(self) -> bool {
        self.channel_count() > 0
    }
}

/// Position of a module: rack (0 = central rack) and slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlotRef {
    pub rack: u8,
    pub slot: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelRef {
    pub rack: u8,
    pub slot: u8,
    pub channel: u8,
}
impl ChannelRef {
    pub fn slot_ref(self) -> SlotRef {
        SlotRef {
            rack: self.rack,
            slot: self.slot,
        }
    }
}
impl std::fmt::Display for ChannelRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "R{}/S{}/CH{}", self.rack, self.slot, self.channel)
    }
}

/// What is physically present in the cabinet, independent of the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstalledHardware {
    pub cpu: ModuleType,
    pub modules: Vec<InstalledModule>,
    pub wiring: Vec<FieldWire>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstalledModule {
    pub position: SlotRef,
    pub module: ModuleType,
    #[serde(default = "yes")]
    pub field_power: bool,
}
fn yes() -> bool {
    true
}

/// A field cable from a module terminal to a component port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldWire {
    pub channel: ChannelRef,
    pub component: Identifier,
    pub port: String,
}

/// The engineer's project configuration of the PLC station.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StationConfig {
    pub name: String,
    pub cpu: ModuleType,
    pub modules: Vec<ConfiguredModule>,
    pub io_map: Vec<IoMapping>,
    #[serde(default)]
    pub network: StationNetworkConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfiguredModule {
    pub position: SlotRef,
    pub module: ModuleType,
    /// Process-image start byte (for byte/bit addressing display) or module start index.
    #[serde(default)]
    pub start_address: u16,
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelConfig {
    pub channel: u8,
    pub mode: ChannelMode,
    #[serde(default)]
    pub diagnostics: bool,
    #[serde(default)]
    pub stop_behavior: OutputStopBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelMode {
    Disabled,
    /// Digital channel enabled.
    Digital,
    Current4To20mA,
    Current0To20mA,
    Voltage0To10V,
}

/// Output behavior while the CPU is not in RUN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputStopBehavior {
    /// De-energize the output (PLC Lab default).
    #[default]
    Off,
    /// Keep the last value written while in RUN.
    HoldLast,
}

/// Links a module channel to a PLC tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IoMapping {
    pub channel: ChannelRef,
    pub tag: TagName,
}

impl StationConfig {
    pub fn module_at(&self, position: SlotRef) -> Option<&ConfiguredModule> {
        self.modules.iter().find(|m| m.position == position)
    }
    pub fn channel_config(&self, channel: ChannelRef) -> Option<&ChannelConfig> {
        self.module_at(channel.slot_ref())?
            .channels
            .iter()
            .find(|c| c.channel == channel.channel)
    }
}
