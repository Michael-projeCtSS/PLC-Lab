//! Trace transport only; canonical observations live in desktop-session.
use crate::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum TraceEventKind {
    SessionLoaded,
    InputStaged,
    ProcessAttached,
    ScanCompleted,
    ScanFaulted,
    RuntimeReset,
    ProcessReset,
    ProcessDetached,
    ExecutionStarted,
    ExecutionPaused,
    ExecutionResumed,
    ExecutionStopped,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TracePageRequest {
    pub contract_version: u32,
    pub session_id: RuntimeSessionId,
    pub after_sequence: Option<TraceSequenceId>,
    pub limit: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraceEntryDto {
    pub sequence: TraceSequenceId,
    pub kind: TraceEventKind,
    pub completed_scans: ExactU64,
    #[ts(type = "string")]
    pub simulation_time_micros: plc_types::PlcTime,
    pub runtime_state: SessionRuntimeState,
    pub execution_mode: ExecutionMode,
    pub execution_run_id: Option<ExecutionRunId>,
    pub variables: Vec<RuntimeVariable>,
    pub function_blocks: Vec<RuntimeFunctionBlock>,
    pub process: Option<ProcessSnapshotDto>,
    pub fault: Option<RuntimeDiagnostic>,
    pub staged_input: Option<RuntimeRelatedVariable>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TracePageDto {
    pub contract_version: u32,
    pub session_id: RuntimeSessionId,
    pub entries: Vec<TraceEntryDto>,
    pub capacity: u16,
    pub oldest_sequence: Option<TraceSequenceId>,
    pub latest_sequence: Option<TraceSequenceId>,
    pub dropped_count: ExactU64,
    pub saturated: bool,
    pub gap: bool,
}
pub(crate) fn trace_typescript(config: &ts_rs::Config) -> String {
    [
        TraceEventKind::decl(config),
        TracePageRequest::decl(config),
        TraceEntryDto::decl(config),
        TracePageDto::decl(config),
    ]
    .into_iter()
    .map(|t| format!("export {t}\n\n"))
    .collect()
}
