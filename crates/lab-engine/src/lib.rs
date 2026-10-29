//! Headless PLC Lab engine: a deterministic world with one PLC participant, a data-driven
//! process scene, station hardware and a virtual network, advanced cycle by cycle.
//!
//! The engine owns cycle execution, causal observations, the compact trace, faults and
//! assessment. It has no dependency on transport DTOs, Tauri or the user interface;
//! errors are typed ([`EngineError`]) and mapped to transport codes by the adapter.

mod assessment;
mod commissioning;
mod error;
mod faults;
mod plc;
mod queries;
mod session;
mod signals;

pub use assessment::{
    AssessmentError, AssessmentOutcome, AssessmentSetup, assess, interface_problems, run_scenario,
};
pub use commissioning::{
    CheckCategory, CheckItem, CheckState, Checklist, FEEDBACK_WAIT_US, SignalMapRow,
};
pub use error::{BindingIssue, EngineError};
pub use plc::{BlockBinding, InputSource, PlcMode, TagBinding, TagRole, compile};
pub use queries::{CauseEffect, Change, Effect, HistoryRow, TrendLayer, TrendSeries, VariableRow};
pub use session::{
    ActiveForce, CycleHeader, CycleObservation, CycleOutcome, DownloadReport, ForceTarget,
    LAYER_APPLIED, LAYER_COMMITTED, LAYER_INPUTS, LAYER_NEXT, LAYER_PROCESS, LAYERS, LabEvent,
    LabEventKind, LabSession, Layer, ModeReason, OperatorAction, SessionConfig,
};
pub use signals::{SignalDescriptor, SignalId, SignalKind, SignalRegistry, SignalValue, ValueType};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
