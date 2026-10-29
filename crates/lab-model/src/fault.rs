//! Typed physical fault injections. Configuration mistakes (wrong range, wrong IP,
//! wrong register offset, wrong mapping) are not injections: exercises seed them into
//! the student's starting project, exactly as they occur during commissioning.

use crate::{
    hardware::{ChannelRef, ModuleType, SlotRef},
    ids::Identifier,
    network::Ipv4,
    scene::{ContactType, OutputStyle},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaultInjection {
    pub id: Identifier,
    pub fault: FaultSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StuckPosition {
    Closed,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FaultSpec {
    /// A digital sensor or contact output is stuck regardless of the process.
    SensorStuck {
        component: Identifier,
        conducting: bool,
    },
    /// The installed device has the other contact type than the design assumes.
    ContactTypeMismatch {
        component: Identifier,
        installed: ContactType,
    },
    /// The installed sensor has another output style (sink/source mismatch).
    OutputStyleMismatch {
        component: Identifier,
        installed: OutputStyle,
    },
    /// Analog transmitter output offset in microamperes (or millivolts).
    AnalogDrift {
        component: Identifier,
        offset: i32,
    },
    /// Analog transmitter output frozen at a value (microamperes or millivolts).
    AnalogStuck {
        component: Identifier,
        value: u32,
    },
    /// Deterministic seeded noise with peak amplitude (microamperes or millivolts).
    AnalogNoise {
        component: Identifier,
        amplitude: u32,
    },
    /// Current loop or signal wire broken at the transmitter.
    LoopBreak {
        component: Identifier,
    },
    ValveStuck {
        component: Identifier,
        position: StuckPosition,
    },
    /// A feedback contact never closes (broken limit switch or auxiliary contact).
    FeedbackMissing {
        component: Identifier,
        port: String,
    },
    MotorOverloadTrip {
        component: Identifier,
    },
    /// A drive left in local (keypad) control: it ignores the run command terminal.
    DriveLocalControl {
        component: Identifier,
    },
    /// The drive trips (latched fault, for example after a mechanical jam). The cause stays
    /// until repaired; the trip must then be acknowledged at the drive.
    DriveTrip {
        component: Identifier,
    },
    /// The drive takes its speed reference from the keypad instead of the terminals.
    DriveKeypadReference {
        component: Identifier,
    },
    /// The drive's mains supply is off: it is not ready.
    DriveNoSupply {
        component: Identifier,
    },
    OpenCircuit {
        channel: ChannelRef,
    },
    ShortToSupply {
        channel: ChannelRef,
    },
    SwappedWires {
        first: ChannelRef,
        second: ChannelRef,
    },
    ModuleMissing {
        position: SlotRef,
    },
    WrongModuleInstalled {
        position: SlotRef,
        installed: ModuleType,
    },
    ModuleInWrongSlot {
        from: SlotRef,
        to: SlotRef,
    },
    FieldPowerMissing {
        position: SlotRef,
    },
    /// The module electronics (backplane) supply is missing; the module does not answer.
    ModulePowerMissing {
        position: SlotRef,
    },
    NetworkLinkDown {
        device: Identifier,
    },
    PlcLinkDown,
    DeviceOffline {
        device: Identifier,
    },
    /// The physical device answers on another address than documented.
    DeviceAddressChanged {
        device: Identifier,
        ip: Ipv4,
    },
}

impl FaultSpec {
    pub fn category(&self) -> FaultCategory {
        match self {
            Self::SensorStuck { .. }
            | Self::ContactTypeMismatch { .. }
            | Self::OutputStyleMismatch { .. } => FaultCategory::DigitalIo,
            Self::AnalogDrift { .. }
            | Self::AnalogStuck { .. }
            | Self::AnalogNoise { .. }
            | Self::LoopBreak { .. } => FaultCategory::AnalogIo,
            Self::ValveStuck { .. } | Self::FeedbackMissing { .. } => FaultCategory::Process,
            Self::MotorOverloadTrip { .. }
            | Self::DriveLocalControl { .. }
            | Self::DriveTrip { .. }
            | Self::DriveKeypadReference { .. }
            | Self::DriveNoSupply { .. } => FaultCategory::Drives,
            Self::OpenCircuit { .. } | Self::ShortToSupply { .. } | Self::SwappedWires { .. } => {
                FaultCategory::Wiring
            }
            Self::ModuleMissing { .. }
            | Self::WrongModuleInstalled { .. }
            | Self::ModuleInWrongSlot { .. }
            | Self::FieldPowerMissing { .. }
            | Self::ModulePowerMissing { .. } => FaultCategory::Hardware,
            Self::NetworkLinkDown { .. }
            | Self::PlcLinkDown
            | Self::DeviceOffline { .. }
            | Self::DeviceAddressChanged { .. } => FaultCategory::Network,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaultCategory {
    DigitalIo,
    AnalogIo,
    Wiring,
    Hardware,
    Process,
    Drives,
    Network,
    Logic,
    Commissioning,
}
