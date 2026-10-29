//! Scene definitions: physical components, process couplings, installed hardware,
//! field wiring, network installation, recommended PLC interface and HMI layout.

use crate::{
    hardware::{InstalledHardware, StationConfig},
    ids::{Identifier, TagName},
    network::NetworkInstallation,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneDefinition {
    pub format: SceneFormat,
    pub schema_version: u32,
    pub id: Identifier,
    pub revision: u32,
    pub title: String,
    pub summary: String,
    pub components: Vec<ComponentDefinition>,
    #[serde(default)]
    pub couplings: Vec<Coupling>,
    pub installation: InstalledHardware,
    /// The correct configuration an engineer would create for this installation.
    pub reference_station: StationConfig,
    #[serde(default)]
    pub network: Option<NetworkInstallation>,
    #[serde(default)]
    pub interface: Vec<InterfaceSignal>,
    #[serde(default)]
    pub layout: Vec<LayoutItem>,
    #[serde(default)]
    pub knowledge_refs: Vec<Identifier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneFormat {
    #[serde(rename = "plclab-scene")]
    Scene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContactType {
    /// Normally open: conducts only while actuated.
    NormallyOpen,
    /// Normally closed: conducts while not actuated.
    NormallyClosed,
}

/// Electrical output style of a sensor or contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputStyle {
    /// Mechanical contact; compatible with either input style when wired accordingly.
    DryContact,
    /// Sourcing (P-switching) three-wire sensor.
    Pnp,
    /// Sinking (N-switching) three-wire sensor.
    Npn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalogSignal {
    Current4To20mA,
    Current0To20mA,
    Voltage0To10V,
}

/// How a current-output transmitter is powered (generic educational model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransmitterWiring {
    /// Loop-powered: the transmitter regulates a current that the input channel (or an
    /// external loop supply) drives through it.
    #[default]
    TwoWire,
    /// Separately powered: the transmitter drives the loop current itself into a passive
    /// input. Voltage-output devices are always separately powered.
    FourWire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LampColor {
    Green,
    Red,
    Amber,
    White,
    Blue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentDefinition {
    pub id: Identifier,
    pub label: String,
    pub kind: ComponentKind,
}

/// Initial physical component catalog. Volumes are millilitres, flows millilitres per
/// second, times milliseconds, currents microamperes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ComponentKind {
    PushButton {
        contact: ContactType,
        #[serde(default)]
        color: Option<LampColor>,
    },
    SelectorSwitch {
        positions: Vec<String>,
        #[serde(default)]
        initial: u8,
    },
    PilotLamp {
        color: LampColor,
    },
    /// Contactor, motor and thermal overload relay. Ports: `coil`, `aux` (NO auxiliary
    /// contact), `overload` (NC overload contact, closed while healthy).
    MotorStarter {
        pickup_ms: u32,
        run_up_ms: u32,
        rated_current_ma: u32,
    },
    /// On/off solenoid valve. Ports: `solenoid`, `openSwitch`, `closedSwitch`.
    OnOffValve {
        travel_ms: u32,
        flow_ml_per_s: u64,
    },
    /// Centrifugal pump driven by a contactor coil. Port: `motor`.
    Pump {
        flow_ml_per_s: u64,
    },
    Tank {
        capacity_ml: u64,
        initial_ml: u64,
        #[serde(default)]
        high_high_ml: Option<u64>,
        #[serde(default)]
        low_low_ml: Option<u64>,
    },
    /// Point level switch mounted at `switchLevelMl`. Port: `contact`. "Actuated" means wet.
    LevelSwitch {
        tank: Identifier,
        switch_level_ml: u64,
        contact: ContactType,
        output: OutputStyle,
    },
    /// Two-wire or four-wire transmitter measuring a tank level. Port: `signal`.
    LevelTransmitter {
        tank: Identifier,
        range_min_ml: u64,
        range_max_ml: u64,
        signal: AnalogSignal,
        #[serde(default)]
        wiring: TransmitterWiring,
    },
    /// Generic proximity or photoelectric presence sensor whose target is set by the
    /// student or a scenario. Port: `output`.
    PresenceSensor {
        output: OutputStyle,
        contact: ContactType,
    },
    /// Generic VFD behavioral model with motor (not a specific product). Ports: `run`
    /// (run command terminal), `ready`, `running` and `remote` (relay outputs); optional
    /// `enable` (run enable terminal), `reset` (fault acknowledge terminal) and `fault`
    /// (fault relay), and `preset` (selects the preset speed). In local control the drive
    /// ignores the run terminal. Speed ramps linearly to the reference over `ramp_ms` for
    /// 0..100 %; deceleration is immediate (approximation).
    VariableFrequencyDrive {
        ramp_ms: u32,
        rated_current_ma: u32,
        /// The drive is ready only while its run enable terminal is energized.
        #[serde(default)]
        enable_terminal: bool,
        /// Fault relay output and fault acknowledge terminal.
        #[serde(default)]
        fault_terminals: bool,
        /// Speed reference when running from the terminals, per mille of rated speed.
        #[serde(default = "full_speed")]
        speed_permille: u32,
        /// Second speed selected by the `preset` terminal.
        #[serde(default)]
        preset_permille: Option<u32>,
        /// Speed when the reference source is set to the keypad, per mille.
        #[serde(default = "keypad_speed")]
        keypad_permille: u32,
    },
}

fn full_speed() -> u32 {
    1000
}
fn keypad_speed() -> u32 {
    300
}

impl ComponentKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::PushButton { .. } => "pushButton",
            Self::SelectorSwitch { .. } => "selectorSwitch",
            Self::PilotLamp { .. } => "pilotLamp",
            Self::MotorStarter { .. } => "motorStarter",
            Self::OnOffValve { .. } => "onOffValve",
            Self::Pump { .. } => "pump",
            Self::Tank { .. } => "tank",
            Self::LevelSwitch { .. } => "levelSwitch",
            Self::LevelTransmitter { .. } => "levelTransmitter",
            Self::PresenceSensor { .. } => "presenceSensor",
            Self::VariableFrequencyDrive { .. } => "variableFrequencyDrive",
        }
    }
    /// Electrical ports exposed by the component.
    pub fn ports(&self) -> Vec<(&'static str, PortKind)> {
        match self {
            Self::PushButton { .. } | Self::LevelSwitch { .. } => {
                vec![("contact", PortKind::DigitalSource)]
            }
            Self::PresenceSensor { .. } => vec![("output", PortKind::DigitalSource)],
            Self::VariableFrequencyDrive {
                enable_terminal,
                fault_terminals,
                preset_permille,
                ..
            } => {
                let mut ports = vec![
                    ("run", PortKind::DigitalSink),
                    ("ready", PortKind::DigitalSource),
                    ("running", PortKind::DigitalSource),
                    ("remote", PortKind::DigitalSource),
                ];
                if *enable_terminal {
                    ports.push(("enable", PortKind::DigitalSink));
                }
                if *fault_terminals {
                    ports.push(("reset", PortKind::DigitalSink));
                    ports.push(("fault", PortKind::DigitalSource));
                }
                if preset_permille.is_some() {
                    ports.push(("preset", PortKind::DigitalSink));
                }
                ports
            }
            Self::SelectorSwitch { positions, .. } => {
                const NAMES: [&str; 4] = ["position0", "position1", "position2", "position3"];
                NAMES
                    .iter()
                    .take(positions.len().min(4))
                    .map(|n| (*n, PortKind::DigitalSource))
                    .collect()
            }
            Self::PilotLamp { .. } => vec![("coil", PortKind::DigitalSink)],
            Self::MotorStarter { .. } => vec![
                ("coil", PortKind::DigitalSink),
                ("aux", PortKind::DigitalSource),
                ("overload", PortKind::DigitalSource),
            ],
            Self::OnOffValve { .. } => vec![
                ("solenoid", PortKind::DigitalSink),
                ("openSwitch", PortKind::DigitalSource),
                ("closedSwitch", PortKind::DigitalSource),
            ],
            Self::Pump { .. } => vec![("motor", PortKind::DigitalSink)],
            Self::Tank { .. } => Vec::new(),
            Self::LevelTransmitter { .. } => vec![("signal", PortKind::AnalogSource)],
        }
    }
    /// For a digital sink port, the quantity that shows the command the device received
    /// (what it was told to do, as opposed to what it then did). `None` for other ports.
    pub fn command_quantity(&self, port: &str) -> Option<&'static str> {
        let sink = self
            .ports()
            .into_iter()
            .any(|(name, kind)| name == port && kind == PortKind::DigitalSink);
        if !sink {
            return None;
        }
        Some(match self {
            Self::PilotLamp { .. } => "lit",
            Self::VariableFrequencyDrive { .. } => match port {
                "enable" => "enableCommand",
                "reset" => "resetCommand",
                "preset" => "presetCommand",
                _ => "command",
            },
            Self::MotorStarter { .. } | Self::OnOffValve { .. } | Self::Pump { .. } => "command",
            Self::PushButton { .. }
            | Self::SelectorSwitch { .. }
            | Self::Tank { .. }
            | Self::LevelSwitch { .. }
            | Self::LevelTransmitter { .. }
            | Self::PresenceSensor { .. } => return None,
        })
    }
    /// Quantities that show received commands, one per digital sink port.
    pub fn commanded_quantities(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = self
            .ports()
            .into_iter()
            .filter_map(|(port, _)| self.command_quantity(port))
            .collect();
        out.dedup();
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PortKind {
    /// Produces a switching field signal (contact or sensor output).
    DigitalSource,
    /// Consumes a 24 V command (coil, lamp, solenoid).
    DigitalSink,
    /// Produces a current or voltage signal.
    AnalogSource,
}

/// Physical process connections between components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Coupling {
    /// The valve admits supply flow into the tank.
    ValveFillsTank { valve: Identifier, tank: Identifier },
    /// The valve drains the tank by gravity.
    ValveDrainsTank { valve: Identifier, tank: Identifier },
    /// The pump draws from the tank.
    PumpDrainsTank { pump: Identifier, tank: Identifier },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TagType {
    Bool,
    Int,
    Real,
    Time,
}

/// A PLC tag the scene's reference configuration maps to I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterfaceSignal {
    pub tag: TagName,
    #[serde(rename = "type")]
    pub tag_type: TagType,
    pub direction: SignalDirection,
    pub description: String,
}

/// HMI placement in a 1000 x 600 logical canvas. Presentation metadata only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutItem {
    pub component: Identifier,
    pub x: u16,
    pub y: u16,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub height: Option<u16>,
}
