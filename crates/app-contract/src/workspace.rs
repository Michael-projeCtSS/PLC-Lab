//! Workspace (live lab session) DTOs: projections of authoritative engine state.
use crate::{
    DiagnosticDto, ExactU64, SignalValueDto, declarations,
    knowledge::{ExerciseDto, PlatformDto},
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
    pub process: Vec<ChangeDto>,
    pub next: Vec<EffectDto>,
    pub fault: Option<DiagnosticDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlarmDto {
    pub component: String,
    pub kind: String,
    pub text: String,
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
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclarationDto {
    pub name: String,
    pub data_type: String,
    pub role: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompileResultDto {
    pub success: bool,
    pub program_name: Option<String>,
    pub diagnostics: Vec<DiagnosticDto>,
    pub declarations: Vec<DeclarationDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildLoadRequest {
    pub contract_version: u32,
    pub workspace_id: ExactU64,
    pub source: String,
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
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariablesDto {
    pub total: u32,
    pub rows: Vec<VariableRowDto>,
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
    pub position: SlotRefDto,
    pub configured: Option<String>,
    pub installed: Option<String>,
    pub configured_name: Option<String>,
    pub installed_name: Option<String>,
    pub product: Option<String>,
    /// "OK" | "MISSING" | "MISMATCH" | "NO_FIELD_POWER" | "UNCONFIGURED"
    #[ts(type = "\"OK\" | \"MISSING\" | \"MISMATCH\" | \"NO_FIELD_POWER\" | \"UNCONFIGURED\"")]
    pub status: String,
    pub status_led: String,
    pub channels: Vec<ChannelViewDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticEntryDto {
    pub time_us: ExactU64,
    pub source: String,
    pub kind: String,
    /// "INCOMING" | "OUTGOING"
    #[ts(type = "\"INCOMING\" | \"OUTGOING\"")]
    pub event: String,
    pub text: String,
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
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkDto {
    pub plc_interface: Option<InterfaceDto>,
    pub plc_link_up: bool,
    pub scans: Vec<ScanViewDto>,
    pub devices: Vec<DeviceViewDto>,
    pub recent: Vec<EventDto>,
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
        CauseEffectDto, AlarmDto, SimulationStatusDto, TraceStatsDto, LiveFrameDto, WorkspaceDto, CompileRequest,
        DeclarationDto, CompileResultDto, BuildLoadRequest, BindingIssueDto, BuildLoadResultDto, PlcModeRequest,
        SimulationRequest, SpeedRequest, OperateActionDto, OperateRequest, ForceRequest, FrameRequest, CycleRequest,
        VariablesRequest, VariableRowDto, VariablesDto, TrendRequest, TrendPointDto, TrendSeriesDto, TrendDto,
        EventsRequest, EventDto, EventsPageDto, ChannelViewDto, ModuleViewDto, DiagnosticEntryDto, ModuleTypeDto,
        HardwareDto, ScanViewDto, DeviceViewDto, NetworkDto, MeasureRequest, MeasurementDto, PingRequest, PingDto,
        EvidenceRequest, DiagnosisRequest, DiagnosisResultDto, RepairRequest, RepairResultDto, CheckRequest,
        CriterionResultDto, GradeReportDto, ExecutionWakeDto
    )
}
