//! Knowledge pack document shapes (pack format 1). Canonical content is JSON with
//! Markdown section bodies; every technical item references provenance records.

use lab_model::{Identifier, profile::FidelityStatement, profile::PlatformProfile};
use serde::{Deserialize, Serialize};

pub const PACK_FORMAT: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackFormatName {
    #[serde(rename = "plclab-knowledge-pack")]
    KnowledgePack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackManifest {
    pub format: PackFormatName,
    pub pack_format: u32,
    pub id: Identifier,
    pub title: String,
    pub version: String,
    pub publisher: String,
    pub license: String,
    pub description: String,
    pub languages: Vec<String>,
    pub updated: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArticleKind {
    Vendor,
    Family,
    Hardware,
    Instruction,
    Protocol,
    Topic,
    Troubleshooting,
    Diagnostic,
}

/// Presentation category shown on result pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    Fundamental,
    VendorSpecific,
    PlcLabSimulation,
    Troubleshooting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Section {
    pub heading: String,
    /// Markdown subset: paragraphs, lists, emphasis, inline code, code blocks, tables.
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fact {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub sources: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Equivalent {
    pub platform: String,
    pub name: String,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupportLevel {
    ExactBehavioral,
    ApproximatedBehavioral,
    Conceptual,
    NotImplemented,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructionDetails {
    pub category: String,
    pub inputs: Vec<Parameter>,
    pub outputs: Vec<Parameter>,
    pub state: String,
    pub retentivity: String,
    pub timing: String,
    pub simulation_support: SupportLevel,
    #[serde(default)]
    pub equivalents: Vec<Equivalent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TroubleshootingDetails {
    pub fault_id: Identifier,
    pub area: String,
    pub target: String,
    pub effect: String,
    pub symptoms: Vec<String>,
    pub evidence: Vec<String>,
    pub diagnostic_method: Vec<String>,
    pub repair: Vec<String>,
    pub explanation: String,
    /// Exercises in which PLC Lab simulates this fault.
    #[serde(default)]
    pub simulated_in: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Article {
    pub id: Identifier,
    pub kind: ArticleKind,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub vendor: Option<Identifier>,
    #[serde(default)]
    pub family: Option<Identifier>,
    #[serde(default)]
    pub protocol: Option<Identifier>,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub sections: Vec<Section>,
    #[serde(default)]
    pub facts: Vec<Fact>,
    #[serde(default)]
    pub fidelity: Option<FidelityStatement>,
    #[serde(default)]
    pub instruction: Option<InstructionDetails>,
    #[serde(default)]
    pub troubleshooting: Option<TroubleshootingDetails>,
    /// Platform simulation profile for `family` articles that PLC Lab can simulate.
    #[serde(default)]
    pub simulation: Option<PlatformProfile>,
    pub sources: Vec<Identifier>,
    #[serde(default)]
    pub related: Vec<Identifier>,
}

impl Article {
    pub fn category(&self) -> Category {
        match self.kind {
            ArticleKind::Troubleshooting => Category::Troubleshooting,
            ArticleKind::Topic | ArticleKind::Protocol => Category::Fundamental,
            ArticleKind::Instruction if self.vendor.is_none() => Category::Fundamental,
            _ => Category::VendorSpecific,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Verification {
    /// Official location and document identity were checked when the record was written.
    Verified,
    /// Document identity is from the author's knowledge; the URL is a vendor portal entry point.
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRecord {
    pub id: Identifier,
    #[serde(default)]
    pub vendor: Option<Identifier>,
    pub title: String,
    pub document_type: String,
    #[serde(default)]
    pub document_number: Option<String>,
    pub url: String,
    pub retrieved: String,
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub notes: String,
    pub confidence: Confidence,
    pub verification: Verification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFile {
    pub sources: Vec<SourceRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LessonSectionType {
    Concept,
    Visual,
    Example,
    Simulation,
    Task,
    Mistake,
    Challenge,
    Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Question {
    pub question: String,
    pub options: Vec<String>,
    pub answer: u8,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonSection {
    #[serde(rename = "type")]
    pub section_type: LessonSectionType,
    pub heading: String,
    pub body: String,
    /// Example ST source shown with the section.
    #[serde(default)]
    pub code: Option<String>,
    /// Interactive scene for simulation sections.
    #[serde(default)]
    pub scene: Option<Identifier>,
    /// Exercise opened by a task section.
    #[serde(default)]
    pub exercise: Option<Identifier>,
    /// Diagnostic challenge question.
    #[serde(default)]
    pub question: Option<Question>,
    /// Visual diagram identifier rendered by the application.
    #[serde(default)]
    pub diagram: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lesson {
    pub id: Identifier,
    pub number: u16,
    pub title: String,
    pub summary: String,
    pub level: String,
    pub minutes: u16,
    pub objectives: Vec<String>,
    pub sections: Vec<LessonSection>,
    pub checkpoint: Vec<Question>,
    #[serde(default)]
    pub knowledge_refs: Vec<Identifier>,
}
