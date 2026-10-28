//! Knowledge, curriculum, exercise catalog and progress DTOs.
use crate::declarations;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackDto {
    pub id: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
    pub path: String,
    /// "LOADED" | "PARTIAL" | "INVALID"
    #[ts(type = "\"LOADED\" | \"PARTIAL\" | \"INVALID\"")]
    pub status: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub articles: u32,
    pub lessons: u32,
    pub exercises: u32,
    pub scenes: u32,
    pub sources: u32,
    pub profiles: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeStatusDto {
    pub packs: Vec<PackDto>,
    pub indexed_items: u32,
    pub search_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeSearchRequest {
    pub contract_version: u32,
    pub query: String,
    /// "FUNDAMENTAL" | "VENDOR_SPECIFIC" | "PLC_LAB_SIMULATION" | "TROUBLESHOOTING"
    pub category: Option<String>,
    pub vendor: Option<String>,
    pub doc_type: Option<String>,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchHitDto {
    pub id: String,
    pub doc_type: String,
    pub category: String,
    pub title: String,
    pub summary: String,
    pub vendor: Option<String>,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdRequest {
    pub contract_version: u32,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionDto {
    pub heading: String,
    pub body: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FactDto {
    pub label: String,
    pub value: String,
    pub sources: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FidelityDto {
    pub level: String,
    pub modeled: Vec<String>,
    pub approximated: Vec<String>,
    pub omitted: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterDto {
    pub name: String,
    pub data_type: String,
    pub description: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EquivalentDto {
    pub platform: String,
    pub name: String,
    pub notes: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructionDto {
    pub category: String,
    pub inputs: Vec<ParameterDto>,
    pub outputs: Vec<ParameterDto>,
    pub state: String,
    pub retentivity: String,
    pub timing: String,
    pub simulation_support: String,
    pub equivalents: Vec<EquivalentDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelatedDto {
    pub id: String,
    pub title: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TroubleshootingDto {
    pub fault_id: String,
    pub area: String,
    pub target: String,
    pub effect: String,
    pub symptoms: Vec<String>,
    pub evidence: Vec<String>,
    pub diagnostic_method: Vec<String>,
    pub repair: Vec<String>,
    pub explanation: String,
    pub simulated_in: Vec<RelatedDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDto {
    pub id: String,
    pub vendor: Option<String>,
    pub title: String,
    pub document_type: String,
    pub document_number: Option<String>,
    pub url: String,
    pub retrieved: String,
    pub applies_to: Vec<String>,
    pub notes: String,
    pub confidence: String,
    pub verification: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArticleDto {
    pub id: String,
    pub kind: String,
    pub category: String,
    pub title: String,
    pub summary: String,
    pub vendor: Option<String>,
    pub family: Option<String>,
    pub protocol: Option<String>,
    pub tier: Option<String>,
    pub tags: Vec<String>,
    pub sections: Vec<SectionDto>,
    pub facts: Vec<FactDto>,
    pub fidelity: Option<FidelityDto>,
    pub instruction: Option<InstructionDto>,
    pub troubleshooting: Option<TroubleshootingDto>,
    pub simulation_profile: Option<String>,
    pub sources: Vec<SourceDto>,
    pub related: Vec<RelatedDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogEntryDto {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub tier: Option<String>,
    pub profile: Option<String>,
    pub area: Option<String>,
    pub children: Vec<CatalogEntryDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogDto {
    pub vendors: Vec<CatalogEntryDto>,
    pub protocols: Vec<CatalogEntryDto>,
    pub fundamentals: Vec<CatalogEntryDto>,
    pub instructions: Vec<CatalogEntryDto>,
    pub troubleshooting: Vec<CatalogEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionDto {
    pub question: String,
    pub options: Vec<String>,
    pub answer: u8,
    pub explanation: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonSectionDto {
    pub section_type: String,
    pub heading: String,
    pub body: String,
    pub code: Option<String>,
    pub scene: Option<String>,
    pub exercise: Option<String>,
    pub question: Option<QuestionDto>,
    pub diagram: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonSummaryDto {
    pub id: String,
    pub number: u16,
    pub title: String,
    pub summary: String,
    pub level: String,
    pub minutes: u16,
    pub completed: bool,
    pub checkpoint_correct: u16,
    pub checkpoint_total: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonDto {
    pub id: String,
    pub number: u16,
    pub title: String,
    pub summary: String,
    pub level: String,
    pub minutes: u16,
    pub objectives: Vec<String>,
    pub sections: Vec<LessonSectionDto>,
    pub checkpoint: Vec<QuestionDto>,
    pub knowledge_refs: Vec<RelatedDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonCompleteRequest {
    pub contract_version: u32,
    pub id: String,
    pub correct: u16,
    pub total: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExerciseSummaryDto {
    pub id: String,
    pub title: String,
    /// "practice" | "troubleshooting" | "commissioning"
    #[ts(type = "\"practice\" | \"troubleshooting\" | \"commissioning\"")]
    pub kind: String,
    #[ts(type = "\"beginner\" | \"intermediate\" | \"advanced\"")]
    pub difficulty: String,
    pub summary: String,
    pub minutes: u16,
    pub scene: String,
    pub platform: String,
    pub attempts: u32,
    pub best_score: u32,
    pub max_score: u32,
    pub passed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequiredTagDto {
    pub tag: String,
    pub data_type: String,
    pub direction: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CriterionDto {
    pub id: String,
    pub title: String,
    pub points: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OptionDto {
    pub id: String,
    pub label: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TroubleshootingBriefDto {
    pub symptom: String,
    pub reported_by: String,
    pub diagnosis_options: Vec<OptionDto>,
    /// Whether the workspace's investigation already contains a correct diagnosis.
    pub diagnosed: bool,
    /// Repair options. Empty until the workspace's investigation has a correct diagnosis,
    /// because the right repair would reveal the root cause.
    pub repairs: Vec<OptionDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExerciseDto {
    pub id: String,
    pub revision: u32,
    pub title: String,
    #[ts(type = "\"practice\" | \"troubleshooting\" | \"commissioning\"")]
    pub kind: String,
    #[ts(type = "\"beginner\" | \"intermediate\" | \"advanced\"")]
    pub difficulty: String,
    pub summary: String,
    pub objectives: Vec<String>,
    pub minutes: u16,
    pub scene: String,
    pub platform: String,
    pub required_interface: Vec<RequiredTagDto>,
    pub instructions: Vec<String>,
    pub hints: Vec<String>,
    pub knowledge_refs: Vec<RelatedDto>,
    pub criteria: Vec<CriterionDto>,
    pub troubleshooting: Option<TroubleshootingBriefDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformDto {
    pub id: String,
    pub display_name: String,
    pub vendor: String,
    pub family: String,
    pub tier: String,
    pub fidelity: FidelityDto,
    pub addressing: String,
    pub analog: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneSummaryDto {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryDto {
    pub lessons: Vec<LessonSummaryDto>,
    pub exercises: Vec<ExerciseSummaryDto>,
    pub platforms: Vec<PlatformDto>,
    pub scenes: Vec<SceneSummaryDto>,
    /// Non-fatal problems found at startup, for example a progress file that could not be
    /// read and was kept aside.
    pub notices: Vec<String>,
}

pub(crate) fn typescript(config: &ts_rs::Config) -> String {
    declarations!(config;
        PackDto, KnowledgeStatusDto, KnowledgeSearchRequest, SearchHitDto, IdRequest, SectionDto, FactDto, FidelityDto,
        ParameterDto, EquivalentDto, InstructionDto, RelatedDto, TroubleshootingDto, SourceDto, ArticleDto,
        CatalogEntryDto, CatalogDto, QuestionDto, LessonSectionDto, LessonSummaryDto, LessonDto, LessonCompleteRequest,
        ExerciseSummaryDto, RequiredTagDto, CriterionDto, OptionDto, TroubleshootingBriefDto, ExerciseDto, PlatformDto,
        SceneSummaryDto, LibraryDto
    )
}
