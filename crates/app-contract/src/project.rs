//! Project and hardware configuration DTOs. `StationDto` has exactly the JSON shape of
//! `lab_model::hardware::StationConfig`; the adapter converts through JSON and the
//! lab-model deserializer remains the authoritative validator.
use crate::{ExactU64, declarations};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlotRefDto {
    pub rack: u8,
    pub slot: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelRefDto {
    pub rack: u8,
    pub slot: u8,
    pub channel: u8,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelConfigDto {
    pub channel: u8,
    /// "disabled" | "digital" | "current4To20mA" | "current0To20mA" | "voltage0To10V"
    #[ts(
        type = "\"disabled\" | \"digital\" | \"current4To20mA\" | \"current0To20mA\" | \"voltage0To10V\""
    )]
    pub mode: String,
    pub diagnostics: bool,
    /// "off" | "holdLast"
    #[ts(type = "\"off\" | \"holdLast\"")]
    pub stop_behavior: String,
    /// Current inputs: the channel supplies the loop (2-wire transmitter); `false` for a
    /// passive input (4-wire transmitter).
    pub loop_supply: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfiguredModuleDto {
    pub position: SlotRefDto,
    pub module: String,
    pub start_address: u16,
    pub channels: Vec<ChannelConfigDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IoMappingDto {
    pub channel: ChannelRefDto,
    pub tag: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterfaceDto {
    pub ip: String,
    pub mask: String,
    pub gateway: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum ModbusDataTypeDto {
    #[serde(rename = "int16")]
    Int16,
    #[serde(rename = "float32")]
    Float32 {
        #[serde(rename = "wordOrder")]
        word_order: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModbusValueDto {
    pub offset: u16,
    pub data_type: ModbusDataTypeDto,
    pub tag: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModbusScanDto {
    pub id: String,
    pub server: String,
    pub port: u16,
    pub unit_id: u8,
    /// "readHoldingRegisters" | "readInputRegisters"
    #[ts(type = "\"readHoldingRegisters\" | \"readInputRegisters\"")]
    pub function: String,
    pub start_address: u16,
    pub quantity: u16,
    pub period_ms: u32,
    pub timeout_ms: u32,
    pub values: Vec<ModbusValueDto>,
    pub status_tag: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StationNetworkDto {
    pub interface: Option<InterfaceDto>,
    pub modbus_tcp: Vec<ModbusScanDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StationDto {
    pub name: String,
    pub cpu: String,
    pub modules: Vec<ConfiguredModuleDto>,
    pub io_map: Vec<IoMappingDto>,
    pub network: StationNetworkDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProgramDocumentDto {
    pub id: String,
    pub name: String,
    pub language: String,
    pub source: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimulationSettingsDto {
    pub cycle_ms: u32,
    pub speed_percent: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    /// "freeLab" | "exercise"
    #[ts(type = "\"freeLab\" | \"exercise\"")]
    pub kind: String,
    pub exercise: Option<String>,
    pub platform: String,
    pub scene: String,
    pub station: StationDto,
    pub programs: Vec<ProgramDocumentDto>,
    pub watch: Vec<String>,
    pub simulation: SimulationSettingsDto,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSummaryDto {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub exercise: Option<String>,
    pub platform: String,
    pub scene: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectListDto {
    pub projects: Vec<ProjectSummaryDto>,
    pub problems: Vec<String>,
    pub directory: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectCreateRequest {
    pub contract_version: u32,
    pub name: String,
    /// Exercise to start, or a free lab defined by `scene` and `platform`.
    pub exercise: Option<String>,
    pub scene: Option<String>,
    pub platform: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSaveRequest {
    pub contract_version: u32,
    pub project: ProjectDto,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceUpdateRequest {
    pub contract_version: u32,
    pub project_id: String,
    pub program_id: String,
    pub source: String,
    pub revision: ExactU64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSaveAsRequest {
    pub contract_version: u32,
    pub project_id: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectImportRequest {
    pub contract_version: u32,
    /// Contents of a `.plclab` file chosen by the user.
    pub text: String,
}
/// A project another launch asked to open (`--open-project <file>`), resolved by the
/// trusted host: the project to open, or why the file could not be opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenRequestDto {
    pub project_id: Option<String>,
    pub name: Option<String>,
    pub problem: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenRequestEventDto {
    pub contract_version: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectExportDto {
    pub file_name: String,
    pub text: String,
}

pub(crate) fn typescript(config: &ts_rs::Config) -> String {
    declarations!(config;
        SlotRefDto, ChannelRefDto, ChannelConfigDto, ConfiguredModuleDto, IoMappingDto, InterfaceDto, ModbusDataTypeDto,
        ModbusValueDto, ModbusScanDto, StationNetworkDto, StationDto, ProgramDocumentDto, SimulationSettingsDto,
        ProjectDto, ProjectSummaryDto, ProjectListDto, ProjectCreateRequest, ProjectSaveRequest, SourceUpdateRequest,
        ProjectSaveAsRequest, ProjectImportRequest, ProjectExportDto, OpenRequestDto, OpenRequestEventDto
    )
}
