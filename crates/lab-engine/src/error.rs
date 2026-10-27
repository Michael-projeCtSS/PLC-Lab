//! Typed engine errors. Mapping to transport error codes belongs to the desktop adapter.

use plc_types::diagnostics::Diagnostic;

/// A problem linking the program's tags to the configured I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingIssue {
    pub tag: String,
    pub location: String,
    pub message: String,
    /// Errors block the download; warnings are reported and the mapping is ignored.
    pub blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// The program does not compile. Diagnostics are student-facing.
    Compile(Vec<Diagnostic>),
    /// The program cannot be linked to the configured I/O.
    Binding(Vec<BindingIssue>),
    /// The hardware configuration was rejected (invalid or wrong CPU).
    Hardware(String),
    /// The scene definition cannot be instantiated.
    Scene(String),
    NoProgram,
    /// The CPU is in FAULT; an explicit PLC reset is required.
    PlcFaulted,
    UnknownComponent(String),
    UnknownSignal(String),
    UnknownChannel(String),
    InvalidAction(String),
    /// An engine invariant was violated. Never a student mistake; fails the session closed.
    Internal(String),
}

impl EngineError {
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Internal(_))
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(d) => write!(f, "The program has {} compiler error(s).", d.len()),
            Self::Binding(issues) => write!(
                f,
                "The program does not match the I/O configuration: {}",
                issues
                    .iter()
                    .filter(|i| i.blocking)
                    .map(|i| i.message.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Hardware(m) | Self::Scene(m) | Self::InvalidAction(m) => f.write_str(m),
            Self::NoProgram => f.write_str("No program is loaded in the PLC."),
            Self::PlcFaulted => {
                f.write_str("The PLC is in FAULT. Reset the PLC before switching to RUN.")
            }
            Self::UnknownComponent(c) => {
                write!(f, "The component '{c}' does not exist in this scene.")
            }
            Self::UnknownSignal(s) => write!(f, "The signal '{s}' does not exist."),
            Self::UnknownChannel(c) => write!(f, "The channel '{c}' does not exist."),
            Self::Internal(m) => write!(f, "Internal simulation error: {m}"),
        }
    }
}
impl std::error::Error for EngineError {}

impl From<process_simulation::SceneError> for EngineError {
    fn from(e: process_simulation::SceneError) -> Self {
        match e {
            process_simulation::SceneError::UnknownComponent(c) => Self::UnknownComponent(c),
            process_simulation::SceneError::TimeReversal => Self::Internal(e.to_string()),
            other => Self::InvalidAction(other.to_string()),
        }
    }
}
impl From<hardware_model::HardwareError> for EngineError {
    fn from(e: hardware_model::HardwareError) -> Self {
        match e {
            hardware_model::HardwareError::Scene(s) => s.into(),
            other => Self::Hardware(other.to_string()),
        }
    }
}
