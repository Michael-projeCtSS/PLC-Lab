//! Workspace (live lab session) DTOs: projections of authoritative engine state.
use crate::{
    DiagnosticDto, ExactU64, SignalValueDto, declarations,
    knowledge::{ExerciseDto, OptionDto, PlatformDto, RelatedDto},
    project::{ChannelRefDto, InterfaceDto, SlotRefDto, StationDto},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceOpenRequest {
    pub contract_version: u32,
    pub project_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignalDto {
    pub id: u32,
    pub path: String,
    pub label: String,
    /// "PLC_INPUT" | "PLC_OUTPUT" | "PLC_LOCAL" | "BLOCK_OUTPUT" | "PHYSICAL" | "CHANNEL"
    #[ts(
        type = "\"PLC_INPUT\" | \"PLC_OUTPUT\" | \"PLC_LOCAL\" | \"BLOCK_OUTPUT\" | \"PHYSICAL\" | \"CHANNEL\""
    )]
    pub kind: String,
    /// "BOOL" | "INT" | "REAL" | "TIME" | "COUNT"
    #[ts(type = "\"BOOL\" | \"INT\" | \"REAL\" | \"TIME\" | \"COUNT\"")]
    pub value_type: String,
    pub unit: Option<String>,
    pub active: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigItemDto {
    pub key: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentDto {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub color: Option<String>,
    pub positions: Vec<String>,
    pub operable: bool,
    pub config: Vec<ConfigItemDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutDto {
    pub component: String,
    pub x: u16,
    pub y: u16,
    pub width: Option<u16>,
    pub height: Option<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterfaceSignalDto {
    pub tag: String,
    pub data_type: String,
    pub direction: String,
    pub description: String,
}
/// Physical process connection drawn as a pipe in the process view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CouplingDto {
    /// "valveFillsTank" | "valveDrainsTank" | "pumpDrainsTank"
    #[ts(type = "\"valveFillsTank\" | \"valveDrainsTank\" | \"pumpDrainsTank\"")]
    pub kind: String,
    /// Upstream component in flow direction.
    pub from: String,
    /// Downstream component in flow direction.
    pub to: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneDto {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub components: Vec<ComponentDto>,
    pub layout: Vec<LayoutDto>,
    pub couplings: Vec<CouplingDto>,
    pub interface: Vec<InterfaceSignalDto>,
    pub has_network: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LedDto {
    pub name: String,
    /// "OFF" | "GREEN" | "GREEN_FLASHING" | "RED" | "RED_FLASHING" | "AMBER"
    #[ts(
        type = "\"OFF\" | \"GREEN\" | \"GREEN_FLASHING\" | \"RED\" | \"RED_FLASHING\" | \"AMBER\""
    )]
    pub state: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlcStatusDto {
    /// "RUN" | "STOP" | "FAULT"
    #[ts(type = "\"RUN\" | \"STOP\" | \"FAULT\"")]
    pub mode: String,
    /// Mode switch position: "RUN" | "STOP"
    pub switch: String,
    pub program_name: Option<String>,
    pub program_revision: Option<ExactU64>,
    pub segment: u32,
    pub scans: ExactU64,
    pub task_time_us: ExactU64,
    pub fault: Option<DiagnosticDto>,
    pub forces: u32,
    pub leds: Vec<LedDto>,
    /// How the latest start initialized program memory (ADR-0038): "WARM" | "COLD".
    #[ts(type = "\"WARM\" | \"COLD\" | null")]
    pub last_start: Option<String>,
    /// Variables of the loaded program that a warm start keeps.
    pub retained_variables: u32,
    /// The next executed scan is the first after a start.
    pub first_scan_pending: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuantityDto {
    pub name: String,
    pub value: SignalValueDto,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentStateDto {
    pub id: String,
    pub quantities: Vec<QuantityDto>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchValueDto {
    pub signal: u32,
    pub value: Option<SignalValueDto>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectDto {
    pub signal: u32,
    pub value: SignalValueDto,
    pub changed: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeDto {
    pub signal: u32,
    pub before: Option<SignalValueDto>,
    pub after: SignalValueDto,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CauseEffectDto {
    pub cycle: ExactU64,
    pub start_us: ExactU64,
    pub end_us: ExactU64,
    pub segment: u32,
    /// "SCANNED" | "FAULTED" | "NOT_SCANNED_STOP" | "NOT_SCANNED_FAULT"
    #[ts(type = "\"SCANNED\" | \"FAULTED\" | \"NOT_SCANNED_STOP\" | \"NOT_SCANNED_FAULT\"")]
    pub outcome: String,
    pub saw: Vec<EffectDto>,
    pub decided: Vec<EffectDto>,
    /// Commands as received by field devices (after output modules and field power).
    pub applied: Vec<EffectDto>,
    pub process: Vec<ChangeDto>,
    pub next: Vec<EffectDto>,
    pub fault: Option<DiagnosticDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlarmDto {
    /// Canonical id of the plant object: a component id, `io.<module>`,
    /// `io.<module>.ch<n>` or `modbus.<connection>`.
    pub component: String,
    /// Generic condition code, for example "HIGH_LEVEL".
    pub kind: String,
    /// Canonical English message.
    pub text: String,
    /// Canonical alarm id, for example "tank.high-level".
    pub id: String,
    pub source_label: String,
    /// "PROCESS" | "HARDWARE" | "NETWORK"
    #[ts(type = "\"PROCESS\" | \"HARDWARE\" | \"NETWORK\"")]
    pub category: String,
    /// "INFORMATION" | "WARNING" | "ALARM" | "CRITICAL"
    #[ts(type = "\"INFORMATION\" | \"WARNING\" | \"ALARM\" | \"CRITICAL\"")]
    pub priority: String,
    /// "NORMAL" | "ACTIVE_UNACKNOWLEDGED" | "ACTIVE_ACKNOWLEDGED" | "RETURNED_UNACKNOWLEDGED"
    #[ts(
        type = "\"NORMAL\" | \"ACTIVE_UNACKNOWLEDGED\" | \"ACTIVE_ACKNOWLEDGED\" | \"RETURNED_UNACKNOWLEDGED\""
    )]
    pub state: String,
    /// Whether the condition holds now: an acknowledged alarm stays active while it does.
    pub condition: bool,
    pub activated_us: Option<ExactU64>,
    pub changed_us: Option<ExactU64>,
    pub acknowledgments: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimulationStatusDto {
    /// "RUNNING" | "PAUSED" | "STOPPED"
    #[ts(type = "\"RUNNING\" | \"PAUSED\" | \"STOPPED\"")]
    pub state: String,
    pub speed_percent: u32,
    pub run_id: Option<ExactU64>,
    pub cycle_us: ExactU64,
    pub failure: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraceStatsDto {
    pub retained_cycles: ExactU64,
    pub dropped_cycles: ExactU64,
    pub used_bytes: ExactU64,
    pub budget_bytes: ExactU64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LiveFrameDto {
    pub workspace_id: ExactU64,
    pub world_time_us: ExactU64,
    pub cycle: ExactU64,
    pub simulation: SimulationStatusDto,
    pub plc: PlcStatusDto,
    pub components: Vec<ComponentStateDto>,
    pub watch: Vec<WatchValueDto>,
    pub cause_effect: Option<CauseEffectDto>,
    pub alarms: Vec<AlarmDto>,
    pub active_diagnostics: u32,
    pub latest_event: Option<ExactU64>,
    pub trace: TraceStatsDto,
    pub signals_revision: u32,
    /// The sequence monitor's view, when the project's exercise defines a sequence.
    pub sequence: Option<SequenceStatusDto>,
}

/// An evidence value: a BOOL or a number.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SequenceValueDto {
    Bool { value: bool },
    Number { value: f64 },
}

/// One comparison of a condition: the signal, what it must be and what it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceConditionRowDto {
    /// Evidence name.
    pub signal: String,
    /// The PLC tag or `component.quantity` it is bound to.
    pub source: String,
    #[ts(type = "\"inputs\" | \"outputs\" | \"process\" | \"next\"")]
    pub layer: String,
    #[ts(type = "\"eq\" | \"ne\" | \"gt\" | \"ge\" | \"lt\" | \"le\"")]
    pub op: String,
    pub expected: SequenceValueDto,
    pub actual: Option<SequenceValueDto>,
    pub satisfied: Option<bool>,
    /// The condition needs this comparison to be FALSE (inside a NOT).
    pub negated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceConditionDto {
    /// Null when evidence is missing (for example the program lacks a tag).
    pub satisfied: Option<bool>,
    pub rows: Vec<SequenceConditionRowDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceStepStateDto {
    pub id: String,
    #[ts(type = "\"INACTIVE\" | \"ACTIVE\" | \"HELD\" | \"COMPLETED\" | \"FAULTED\"")]
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequencePermissiveStateDto {
    pub id: String,
    pub condition: SequenceConditionDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceTransitionStateDto {
    pub id: String,
    /// The transition leaves the active step.
    pub from_active: bool,
    pub condition: SequenceConditionDto,
    pub permissives: Vec<SequencePermissiveStateDto>,
    /// Active interlocks that block it.
    pub blocked_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceInterlockStateDto {
    pub id: String,
    pub applies: bool,
    pub active: bool,
    pub condition: SequenceConditionDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceObservedDto {
    pub signal: String,
    pub source: String,
    #[ts(type = "\"inputs\" | \"outputs\" | \"process\" | \"next\"")]
    pub layer: String,
    pub value: Option<SequenceValueDto>,
}

/// One monitor event; texts are composed by the view from the code and localized content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceEventDto {
    pub seq: ExactU64,
    pub time_us: ExactU64,
    pub cycle: ExactU64,
    #[ts(
        type = "\"runStarted\" | \"transitionCompleted\" | \"sequenceStarted\" | \"sequenceCompleted\" | \"sequenceAborted\" | \"startBlocked\" | \"transitionBlocked\" | \"interlockActivated\" | \"interlockCleared\" | \"held\" | \"resumed\" | \"stepTimeout\" | \"faultCleared\" | \"unknownStep\" | \"undefinedTransition\" | \"conditionNotMet\" | \"permissiveViolated\" | \"interlockViolated\" | \"progressDuringHold\" | \"progressAfterFault\""
    )]
    pub code: String,
    /// A deviation or a step timeout.
    pub fault: bool,
    pub step: Option<String>,
    pub from: Option<String>,
    pub transition: Option<String>,
    pub interlock: Option<String>,
    pub permissives: Vec<String>,
    pub interlocks: Vec<String>,
    pub step_value: Option<i16>,
    #[ts(type = "\"WARM\" | \"COLD\" | \"ATTACHED\" | null")]
    pub start: Option<String>,
    pub values: Vec<SequenceObservedDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceStatusDto {
    pub sequence_id: String,
    #[ts(
        type = "\"UNBOUND\" | \"STOPPED\" | \"IDLE\" | \"RUNNING\" | \"HELD\" | \"COMPLETED\" | \"FAULTED\""
    )]
    pub state: String,
    pub active_step: Option<String>,
    pub previous_step: Option<String>,
    pub step_value: Option<i16>,
    pub elapsed_us: ExactU64,
    pub timeout_us: Option<ExactU64>,
    /// Fault reason code ("stepTimeout" or a deviation code).
    pub fault: Option<String>,
    pub fault_step: Option<String>,
    pub steps: Vec<SequenceStepStateDto>,
    pub transitions: Vec<SequenceTransitionStateDto>,
    pub interlocks: Vec<SequenceInterlockStateDto>,
    /// The newest events, oldest first (at most 40).
    pub history: Vec<SequenceEventDto>,
    pub total_events: ExactU64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceDto {
    pub workspace_id: ExactU64,
    pub project_id: String,
    pub scene: SceneDto,
    pub platform: PlatformDto,
    pub signals: Vec<SignalDto>,
    pub exercise: Option<ExerciseDto>,
    pub frame: LiveFrameDto,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompileRequest {
    pub contract_version: u32,
    pub source: String,
    /// The project library documents compiled with `source` (ADR-0045).
    #[serde(default)]
    pub library: Vec<SourceDocumentDto>,
}
/// A project library document: FUNCTIONs and FUNCTION_BLOCKs (ADR-0045).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDocumentDto {
    /// The project's program document id.
    pub id: String,
    pub source: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclarationDto {
    pub name: String,
    pub data_type: String,
    pub role: String,
    /// Declared in VAR RETAIN.
    pub retained: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompileResultDto {
    pub success: bool,
    pub program_name: Option<String>,
    pub diagnostics: Vec<DiagnosticDto>,
    pub declarations: Vec<DeclarationDto>,
    /// The document's user functions and function blocks (ADR-0044).
    pub pous: Vec<PouDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PouParameterDto {
    pub name: String,
    pub data_type: String,
    #[ts(type = "\"Input\" | \"Output\" | \"InOut\"")]
    pub direction: String,
    /// The comment after the declaration.
    pub description: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PouDto {
    pub name: String,
    #[ts(type = "\"FUNCTION\" | \"FUNCTION_BLOCK\"")]
    pub kind: String,
    pub return_type: Option<String>,
    pub parameters: Vec<PouParameterDto>,
    /// The comment block before the POU.
    pub description: Option<String>,
    /// First line of the POU in the document.
    pub line: u32,
    /// The library document that defines it; `None` for the main program (ADR-0045).
    pub document: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildLoadRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub source: String,
    /// The project library documents compiled with `source` (ADR-0045).
    #[serde(default)]
    pub library: Vec<SourceDocumentDto>,
    pub revision: ExactU64,
    pub station: Option<StationDto>,
    /// Basic mode: switch the PLC to RUN and start the simulation after a successful load.
    pub run_after_load: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingIssueDto {
    pub tag: String,
    pub location: String,
    pub message: String,
    pub blocking: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildLoadResultDto {
    /// "LOADED" | "COMPILE_FAILED" | "BINDING_FAILED" | "HARDWARE_REJECTED" | "SUPERSEDED"
    #[ts(
        type = "\"LOADED\" | \"COMPILE_FAILED\" | \"BINDING_FAILED\" | \"HARDWARE_REJECTED\" | \"SUPERSEDED\""
    )]
    pub outcome: String,
    pub diagnostics: Vec<DiagnosticDto>,
    pub binding_issues: Vec<BindingIssueDto>,
    pub message: Option<String>,
    pub signals: Vec<SignalDto>,
    pub frame: LiveFrameDto,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlcModeRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    /// "RUN" | "STOP"
    #[ts(type = "\"RUN\" | \"STOP\"")]
    pub mode: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimulationRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    /// "RUN" | "PAUSE" | "STEP" | "STOP"
    #[ts(type = "\"RUN\" | \"PAUSE\" | \"STEP\" | \"STOP\"")]
    pub command: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpeedRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub speed_percent: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum OperateActionDto {
    #[serde(rename = "press")]
    Press,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "selector")]
    Selector { position: u8 },
    #[serde(rename = "presence")]
    Presence { present: bool },
    /// Open or close a heated vessel's door (ADR-0047).
    #[serde(rename = "door")]
    Door { open: bool },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperateRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub component: String,
    pub action: OperateActionDto,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForceRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub signal: u32,
    pub value: Option<SignalValueDto>,
}
/// An active force: the signal, what it replaces and the forced value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForceEntryDto {
    pub signal: u32,
    pub path: String,
    pub label: String,
    #[ts(type = "\"INPUT_IMAGE\" | \"OUTPUT_IMAGE\"")]
    pub target: String,
    pub value: SignalValueDto,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForceListDto {
    pub forces: Vec<ForceEntryDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub watch: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CycleRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub cycle: ExactU64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariablesRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub filter: Option<String>,
    pub offset: u32,
    pub limit: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariableRowDto {
    pub signal: u32,
    pub name: String,
    pub kind: String,
    pub scan_value: Option<SignalValueDto>,
    pub committed: Option<SignalValueDto>,
    pub staged: Option<SignalValueDto>,
    pub source: Option<String>,
    pub quality: Option<String>,
    pub forced: Option<SignalValueDto>,
    /// Kept by a warm start (VAR RETAIN, ADR-0038).
    pub retained: bool,
    /// The value a start initializes the variable with.
    pub initial: Option<SignalValueDto>,
    /// What the latest start did to the variable: "KEPT" | "INITIALIZED".
    #[ts(type = "\"KEPT\" | \"INITIALIZED\" | null")]
    pub restart_effect: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariablesDto {
    pub total: u32,
    pub rows: Vec<VariableRowDto>,
}
/// Live rows of a watch table: the signals by id, at most 64.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub signals: Vec<u32>,
}
/// One watched signal as the runtime holds it: canonical identity, value, type, quality,
/// where it comes from, the PLC's scan input, committed value and next input, and an
/// active force.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchEntryDto {
    pub signal: u32,
    pub path: String,
    pub label: String,
    pub kind: String,
    pub value_type: String,
    pub unit: Option<String>,
    pub active: bool,
    pub value: Option<SignalValueDto>,
    pub scan_value: Option<SignalValueDto>,
    pub committed: Option<SignalValueDto>,
    pub staged: Option<SignalValueDto>,
    pub source: Option<String>,
    pub quality: Option<String>,
    pub forced: Option<SignalValueDto>,
    /// Kept by a warm start (VAR RETAIN, ADR-0038).
    pub retained: bool,
    /// The value a start initializes the variable with.
    pub initial: Option<SignalValueDto>,
    /// What the latest start did to the variable: "KEPT" | "INITIALIZED".
    #[ts(type = "\"KEPT\" | \"INITIALIZED\" | null")]
    pub restart_effect: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchDto {
    pub world_time_us: ExactU64,
    pub cycle: ExactU64,
    pub rows: Vec<WatchEntryDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrendRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub signals: Vec<u32>,
    pub from_us: ExactU64,
    pub to_us: ExactU64,
    pub max_points: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrendPointDto {
    pub t: ExactU64,
    pub value: SignalValueDto,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrendSeriesDto {
    pub signal: u32,
    /// "INPUTS" | "COMMITTED" | "PROCESS"
    #[ts(type = "\"INPUTS\" | \"COMMITTED\" | \"PROCESS\"")]
    pub layer: String,
    pub points: Vec<TrendPointDto>,
    pub decimated: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrendDto {
    pub series: Vec<TrendSeriesDto>,
    pub oldest_us: Option<ExactU64>,
    pub latest_us: Option<ExactU64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventsRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub after: Option<ExactU64>,
    pub limit: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventDto {
    pub seq: ExactU64,
    pub time_us: ExactU64,
    pub cycle: ExactU64,
    pub kind: String,
    /// "info" | "warning" | "error"
    #[ts(type = "\"info\" | \"warning\" | \"error\"")]
    pub severity: String,
    pub text: String,
    pub source: Option<String>,
    /// `text` as a message for the viewer's language, where the event has one.
    pub message: Option<MessageDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventsPageDto {
    pub events: Vec<EventDto>,
    pub gap: bool,
    pub dropped: ExactU64,
    pub latest: Option<ExactU64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelViewDto {
    pub channel: u8,
    /// Stable channel identity `<moduleId>.ch<n>` (configured modules only).
    pub channel_id: Option<String>,
    pub mode: String,
    pub tag: Option<String>,
    pub address: String,
    pub value: Option<SignalValueDto>,
    pub quality: Option<String>,
    pub output: Option<bool>,
    pub led: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleViewDto {
    /// Stable id of the configured module at this position.
    pub module_id: Option<String>,
    pub position: SlotRefDto,
    pub configured: Option<String>,
    pub installed: Option<String>,
    pub configured_name: Option<String>,
    pub installed_name: Option<String>,
    pub product: Option<String>,
    /// "OK" | "MISSING" | "MISMATCH" | "NO_FIELD_POWER" | "NO_MODULE_POWER" | "UNCONFIGURED"
    #[ts(
        type = "\"OK\" | \"MISSING\" | \"MISMATCH\" | \"NO_FIELD_POWER\" | \"NO_MODULE_POWER\" | \"UNCONFIGURED\""
    )]
    pub status: String,
    pub status_led: String,
    pub channels: Vec<ChannelViewDto>,
}
/// A structured detail of a diagnostic entry (for example the mode before and after).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticDetailDto {
    pub name: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticEntryDto {
    pub time_us: ExactU64,
    pub source: String,
    pub kind: String,
    /// "INCOMING" | "OUTGOING" | "EVENT"
    #[ts(type = "\"INCOMING\" | \"OUTGOING\" | \"EVENT\"")]
    pub event: String,
    pub text: String,
    /// The module slot the entry is about (module and channel sources), as typed
    /// identity: consumers never parse `source`.
    pub slot: Option<SlotRefDto>,
    /// The channel the entry is about (channel sources).
    pub channel: Option<ChannelRefDto>,
    /// Stable id of the configured module at `slot`.
    pub module_id: Option<String>,
    #[ts(type = "\"error\" | \"warning\" | \"info\"")]
    pub severity: String,
    #[ts(type = "\"CPU\" | \"HARDWARE\" | \"NETWORK\"")]
    pub category: String,
    pub details: Vec<DiagnosticDetailDto>,
    pub active: bool,
    pub cleared_us: Option<ExactU64>,
}
/// The CPU diagnostic buffer, newest first, filtered and paged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticBufferRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub offset: u32,
    pub limit: u16,
    pub severity: Option<String>,
    pub category: Option<String>,
    pub module_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticBufferDto {
    pub capacity: u32,
    pub recorded: ExactU64,
    pub matching: u32,
    pub entries: Vec<DiagnosticEntryDto>,
}
/// A warm or cold restart of the running CPU (ADR-0038): "WARM" | "COLD".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlcRestartRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    #[ts(type = "\"WARM\" | \"COLD\"")]
    pub kind: String,
}
/// The plant's alarms (ADR-0037) and their history, newest first, in pages. The filters
/// apply to both: a priority, a state (of an alarm, or the state a transition led to) and
/// a case-insensitive part of the source id or label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlarmsRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub offset: u32,
    pub limit: u16,
    pub priority: Option<String>,
    pub state: Option<String>,
    pub source: Option<String>,
}
/// One recorded alarm state change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlarmEventDto {
    pub seq: ExactU64,
    pub time_us: ExactU64,
    pub alarm: String,
    pub code: String,
    pub source: String,
    pub source_label: String,
    #[ts(type = "\"PROCESS\" | \"HARDWARE\" | \"NETWORK\"")]
    pub category: String,
    #[ts(type = "\"INFORMATION\" | \"WARNING\" | \"ALARM\" | \"CRITICAL\"")]
    pub priority: String,
    pub message: String,
    /// "ACTIVATED" | "ACKNOWLEDGED" | "RETURNED_TO_NORMAL" | "WITHDRAWN"
    #[ts(type = "\"ACTIVATED\" | \"ACKNOWLEDGED\" | \"RETURNED_TO_NORMAL\" | \"WITHDRAWN\"")]
    pub transition: String,
    #[ts(
        type = "\"NORMAL\" | \"ACTIVE_UNACKNOWLEDGED\" | \"ACTIVE_ACKNOWLEDGED\" | \"RETURNED_UNACKNOWLEDGED\""
    )]
    pub from: String,
    #[ts(
        type = "\"NORMAL\" | \"ACTIVE_UNACKNOWLEDGED\" | \"ACTIVE_ACKNOWLEDGED\" | \"RETURNED_UNACKNOWLEDGED\""
    )]
    pub to: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlarmsDto {
    pub world_time_us: ExactU64,
    /// Alarms configured for the plant.
    pub configured: u32,
    /// Alarms active or waiting for acknowledgment, highest priority and newest first.
    pub alarms: Vec<AlarmDto>,
    pub capacity: u32,
    /// Transitions recorded since the world started, including those no longer kept.
    pub recorded: ExactU64,
    /// History entries kept.
    pub kept: u32,
    /// Kept history entries that pass the filters.
    pub matching: u32,
    pub history: Vec<AlarmEventDto>,
}
/// Acknowledge the listed alarms (all or none), or every alarm waiting for it when
/// `alarms` is null.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlarmAckRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub alarms: Option<Vec<String>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlarmAckResultDto {
    pub acknowledged: u32,
    pub frame: LiveFrameDto,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleTypeDto {
    pub id: String,
    pub name: String,
    pub product: Option<String>,
    pub channels: u8,
    pub class: String,
}
/// A user-facing message: a stable code, its named values and the canonical English
/// text. The interface words the code in the viewer's language and falls back to `text`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessageDto {
    pub code: String,
    pub text: String,
    pub args: Vec<MessageArgDto>,
}
/// A value of a message: data shown as it is, a vocabulary term, or a nested message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessageArgDto {
    pub name: String,
    /// The value as shown in English.
    pub text: String,
    /// The vocabulary code of a term; `None` for data and nested messages.
    pub term: Option<String>,
    pub message: Option<Box<MessageDto>>,
}

/// One commissioning check, evaluated by the engine from the installation's observable
/// state and the I/O tests performed in this session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckItemDto {
    pub id: String,
    pub title: String,
    /// "OK" | "ATTENTION" | "FAULT" | "UNTESTED" | "NOT_APPLICABLE"
    pub state: String,
    pub findings: Vec<String>,
    /// What to do or which tool shows it; empty when nothing applies.
    pub how: String,
    /// `findings` as messages the interface words in the viewer's language.
    pub finding_messages: Vec<MessageDto>,
    /// `how` as a message; `None` when nothing applies.
    pub how_message: Option<MessageDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckCategoryDto {
    /// "power" | "hardware" | "network" | "inputs" | "outputs" | "drives" | "process"
    pub id: String,
    pub title: String,
    /// The most severe item state.
    pub state: String,
    pub items: Vec<CheckItemDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissioningDto {
    pub categories: Vec<CheckCategoryDto>,
    /// Seconds an actuator must stay energized before missing feedback is reported.
    pub feedback_wait_s: u32,
}

/// A difference between the project, the installation and the drawing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareFindingDto {
    /// "MODULE_MISSING" | "UNEXPECTED_MODULE" | "WRONG_MODULE_TYPE" | "WRONG_SLOT" |
    /// "MODULE_POWER_MISSING" | "FIELD_POWER_MISSING" | "CHANNEL_MODE_MISMATCH" |
    /// "LOOP_SUPPLY_MISMATCH" | "ADDRESS_OVERLAP" | "SIGNAL_BINDING_MISMATCH"
    pub category: String,
    /// "warning" | "error"
    #[ts(type = "\"warning\" | \"error\"")]
    pub severity: String,
    pub slot: Option<SlotRefDto>,
    pub channel: Option<ChannelRefDto>,
    pub module_id: Option<String>,
    pub expected: Option<String>,
    pub detected: Option<String>,
    pub message: String,
    /// `message`, `expected` and `detected` as messages for the viewer's language.
    pub detail: MessageDto,
    pub expected_message: Option<MessageDto>,
    pub detected_message: Option<MessageDto>,
}
/// A mapped PLC signal followed through the installation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignalMapRowDto {
    pub tag: String,
    pub channel: ChannelRefDto,
    pub channel_id: Option<String>,
    pub address: String,
    pub value: Option<SignalValueDto>,
    pub quality: Option<String>,
    pub output: Option<bool>,
    /// The device and port connected to this channel on the drawing.
    pub drawn_device: Option<String>,
    pub drawn_port: Option<String>,
    /// The drawn device's own state.
    pub device_state: Option<String>,
    pub device_state_message: Option<MessageDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareDto {
    pub station: StationDto,
    pub cpu: String,
    pub cpu_name: String,
    pub cpu_product: Option<String>,
    pub cpu_leds: Vec<LedDto>,
    pub modules: Vec<ModuleViewDto>,
    pub diagnostics: Vec<DiagnosticEntryDto>,
    pub catalog: Vec<ModuleTypeDto>,
    /// Configured against actual against drawing, derived by the backend.
    pub findings: Vec<HardwareFindingDto>,
    pub signals: Vec<SignalMapRowDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanViewDto {
    pub id: String,
    pub server: String,
    pub unit_id: u8,
    pub status: String,
    pub connected: bool,
    pub requests: ExactU64,
    pub responses: ExactU64,
    pub timeouts: ExactU64,
    pub exceptions: ExactU64,
    pub last_round_trip_us: Option<ExactU64>,
    pub registers: Vec<u16>,
    /// The request the PLC sends for this connection, from the downloaded configuration.
    pub request: ModbusRequestDto,
    /// The configured values, decoded by the backend from the last good registers.
    pub values: Vec<ScanValueDto>,
    pub last_request_us: Option<ExactU64>,
    pub last_response_us: Option<ExactU64>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModbusRequestDto {
    /// "readHoldingRegisters" | "readInputRegisters"
    #[ts(type = "\"readHoldingRegisters\" | \"readInputRegisters\"")]
    pub function: String,
    pub function_code: u8,
    /// Zero-based protocol (PDU) address of the first register.
    pub start_address: u16,
    pub quantity: u16,
    pub port: u16,
    pub period_ms: u32,
    pub timeout_ms: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanValueDto {
    pub tag: String,
    /// Offset from the request start address, in registers.
    pub offset: u16,
    /// "int16" | "float32"
    #[ts(type = "\"int16\" | \"float32\"")]
    pub data_type: String,
    /// Float32 only: "highWordFirst" | "lowWordFirst"
    #[ts(type = "\"highWordFirst\" | \"lowWordFirst\" | null")]
    pub word_order: Option<String>,
    /// Decoded value, `None` before the first good response.
    pub value: Option<String>,
}
/// One Modbus TCP message or communication result, as the PLC's network stack saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrafficDto {
    pub seq: ExactU64,
    pub time_us: ExactU64,
    /// The connection (scan) id.
    pub connection: String,
    pub transaction: u16,
    pub peer: String,
    /// "request" | "response" | "exception" | "timeout" | "noRoute" | "arpUnresolved" |
    /// "refused" | "addressConflict"
    #[ts(
        type = "\"request\" | \"response\" | \"exception\" | \"timeout\" | \"noRoute\" | \"arpUnresolved\" | \"refused\" | \"addressConflict\""
    )]
    pub kind: String,
    pub unit_id: Option<u8>,
    pub function_code: Option<u8>,
    pub address: Option<u16>,
    pub quantity: Option<u16>,
    pub round_trip_us: Option<ExactU64>,
    pub exception_code: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceViewDto {
    pub id: String,
    pub label: String,
    pub ip: String,
    pub mask: String,
    pub link_up: bool,
    pub online: bool,
    pub modbus_server: bool,
    pub requests_served: ExactU64,
    pub last_activity_us: Option<ExactU64>,
    /// Whether the device address lies in the PLC interface's subnet (`None` without one).
    pub same_subnet: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkDto {
    pub plc_interface: Option<InterfaceDto>,
    pub plc_link_up: bool,
    pub scans: Vec<ScanViewDto>,
    pub devices: Vec<DeviceViewDto>,
    /// The newest Modbus TCP messages and results, oldest first (bounded).
    pub traffic: Vec<TrafficDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasureRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub channel: ChannelRefDto,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasurementDto {
    /// "VOLTAGE" | "CURRENT" | "NO_MODULE"
    #[ts(type = "\"VOLTAGE\" | \"CURRENT\" | \"NO_MODULE\"")]
    pub kind: String,
    #[ts(type = "number")]
    pub value: i64,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PingRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub ip: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PingDto {
    pub result: String,
    pub round_trip_us: Option<ExactU64>,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub evidence: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosisRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub option: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosisResultDto {
    pub correct: bool,
    pub message: String,
    pub attempts: u32,
    /// The repair options, unlocked by a correct diagnosis; empty otherwise.
    pub repairs: Vec<OptionDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepairRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub repair: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepairResultDto {
    pub message: String,
    pub frame: LiveFrameDto,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub source: String,
    /// The project library documents compiled with `source` (ADR-0045).
    #[serde(default)]
    pub library: Vec<SourceDocumentDto>,
    pub station: Option<StationDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CriterionResultDto {
    pub id: String,
    pub title: String,
    pub points: u32,
    pub earned: u32,
    pub passed: bool,
    pub feedback: String,
    pub detail: String,
}
/// Troubleshooting explanation, present only after a passed verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DebriefDto {
    pub root_cause: String,
    pub evidence: Vec<String>,
    pub repair: String,
    pub reasoning: String,
    /// Knowledge specific to this root cause, revealed only with the debrief.
    pub references: Vec<RelatedDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GradeReportDto {
    /// "GRADED" | "COMPILE_FAILED" | "INTERFACE_MISMATCH" | "BINDING_FAILED"
    #[ts(type = "\"GRADED\" | \"COMPILE_FAILED\" | \"INTERFACE_MISMATCH\" | \"BINDING_FAILED\"")]
    pub outcome: String,
    pub score: u32,
    pub max_score: u32,
    pub passed: bool,
    pub criteria: Vec<CriterionResultDto>,
    pub diagnostics: Vec<DiagnosticDto>,
    pub problems: Vec<String>,
    pub debrief: Option<DebriefDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionWakeDto {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub run_id: ExactU64,
    pub cycle: ExactU64,
}

pub(crate) fn typescript(config: &ts_rs::Config) -> String {
    declarations!(config;
        WorkspaceRequest, WorkspaceOpenRequest, SignalDto, ConfigItemDto, ComponentDto, LayoutDto, CouplingDto, InterfaceSignalDto,
        SceneDto, LedDto, PlcStatusDto, QuantityDto, ComponentStateDto, WatchValueDto, EffectDto, ChangeDto,
        CauseEffectDto, AlarmDto, SimulationStatusDto, TraceStatsDto, SequenceValueDto, SequenceConditionRowDto,
        SequenceConditionDto, SequenceStepStateDto, SequencePermissiveStateDto, SequenceTransitionStateDto,
        SequenceInterlockStateDto, SequenceObservedDto, SequenceEventDto, SequenceStatusDto, LiveFrameDto, WorkspaceDto, CompileRequest, SourceDocumentDto,
        DeclarationDto, CompileResultDto, PouParameterDto, PouDto, BuildLoadRequest, BindingIssueDto, BuildLoadResultDto, PlcModeRequest,
        SimulationRequest, SpeedRequest, OperateActionDto, OperateRequest, ForceRequest, ForceEntryDto, ForceListDto, FrameRequest, CycleRequest,
        VariablesRequest, VariableRowDto, VariablesDto, WatchRequest, WatchEntryDto, WatchDto, TrendRequest, TrendPointDto, TrendSeriesDto, TrendDto,
        EventsRequest, EventDto, EventsPageDto, ChannelViewDto, ModuleViewDto, DiagnosticDetailDto, DiagnosticEntryDto, DiagnosticBufferRequest, DiagnosticBufferDto,
        PlcRestartRequest, AlarmsRequest, AlarmEventDto, AlarmsDto, AlarmAckRequest, AlarmAckResultDto, ModuleTypeDto,
        MessageDto, MessageArgDto, CheckItemDto, CheckCategoryDto, CommissioningDto, HardwareFindingDto, SignalMapRowDto, HardwareDto, ScanViewDto, ModbusRequestDto, ScanValueDto, TrafficDto, DeviceViewDto, NetworkDto, MeasureRequest, MeasurementDto, PingRequest, PingDto,
        EvidenceRequest, DiagnosisRequest, DiagnosisResultDto, RepairRequest, RepairResultDto, CheckRequest,
        CriterionResultDto, DebriefDto, GradeReportDto, ExecutionWakeDto
    )
}
