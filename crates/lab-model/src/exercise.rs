//! Data-driven exercises: scenario scripts, evidence signal bindings, behavioral criteria
//! and troubleshooting metadata. Evaluation lives in `grading-core`.

use crate::{
    fault::FaultInjection,
    hardware::StationConfig,
    ids::{Identifier, TagName},
    scene::{SignalDirection, TagType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExerciseFormat {
    #[serde(rename = "plclab-exercise")]
    Exercise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExerciseKind {
    Practice,
    Troubleshooting,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExerciseDefinition {
    pub format: ExerciseFormat,
    pub schema_version: u32,
    pub id: Identifier,
    pub revision: u32,
    pub title: String,
    pub kind: ExerciseKind,
    pub difficulty: Difficulty,
    pub summary: String,
    pub objectives: Vec<String>,
    #[serde(default)]
    pub prerequisites: Vec<Identifier>,
    pub estimated_minutes: u16,
    pub scene: Identifier,
    #[serde(default)]
    pub platform: Option<Identifier>,
    pub starter_program: String,
    /// Starting hardware configuration. Absent: the scene reference station.
    #[serde(default)]
    pub starter_station: Option<StationConfig>,
    pub required_interface: Vec<RequiredTag>,
    pub instructions: Vec<String>,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub knowledge_refs: Vec<Identifier>,
    pub evidence: Vec<EvidenceSignal>,
    pub scenarios: Vec<Scenario>,
    pub criteria: Vec<Criterion>,
    #[serde(default)]
    pub troubleshooting: Option<TroubleshootingSetup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequiredTag {
    pub tag: TagName,
    #[serde(rename = "type")]
    pub tag_type: TagType,
    pub direction: SignalDirection,
}

/// A named fact the grader may reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceSignal {
    pub name: Identifier,
    pub source: EvidenceSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EvidenceSource {
    /// A PLC tag: inputs come from the scan input image, outputs from the commit.
    PlcTag { tag: TagName },
    /// A physical component quantity after the process interval.
    Component {
        component: Identifier,
        quantity: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scenario {
    pub id: Identifier,
    pub title: String,
    pub duration_ms: u32,
    #[serde(default)]
    pub actions: Vec<TimedAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimedAction {
    pub at_ms: u32,
    pub action: ScenarioAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlcModeRequest {
    Run,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ScenarioAction {
    Press {
        component: Identifier,
    },
    Release {
        component: Identifier,
    },
    SetSelector {
        component: Identifier,
        position: u8,
    },
    SetPresence {
        component: Identifier,
        present: bool,
    },
    SetPlcMode {
        mode: PlcModeRequest,
    },
    InjectFault {
        fault: FaultInjection,
    },
    ClearFault {
        id: Identifier,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Criterion {
    pub id: Identifier,
    pub title: String,
    pub points: u32,
    pub scenario: Identifier,
    pub rule: Rule,
    pub on_fail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Rule {
    /// Holds in every cycle at or after `fromMs`.
    Always {
        condition: Condition,
        #[serde(default)]
        from_ms: u32,
    },
    /// Never holds in any cycle.
    Never { condition: Condition },
    /// Holds in at least one cycle that starts no later than `byMs`.
    Eventually { condition: Condition, by_ms: u32 },
    /// Every time `trigger` becomes true, `response` must hold within `withinMs`.
    RespondsWithin {
        trigger: Condition,
        response: Condition,
        within_ms: u32,
    },
    /// Holds in the cycle that contains `atMs`.
    StateAt { at_ms: u32, condition: Condition },
    /// No PLC runtime fault occurred.
    NoPlcFault,
    /// No process event of this kind occurred.
    EventAbsent { event: ProcessEventKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessEventKind {
    Overflow,
    DryRun,
    HighHigh,
    LowLow,
    MotorOverload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceLayer {
    /// PLC scan input image.
    Inputs,
    /// Values committed by the scan.
    Outputs,
    /// Physical state after the process interval.
    Process,
    /// Inputs sampled for the following scan.
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Comparison {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Condition {
    All {
        of: Vec<Condition>,
    },
    Any {
        of: Vec<Condition>,
    },
    Not {
        of: Box<Condition>,
    },
    Compare {
        signal: Identifier,
        layer: EvidenceLayer,
        op: Comparison,
        value: Literal,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Literal {
    Bool(bool),
    Number(f64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TroubleshootingSetup {
    pub symptom: String,
    pub reported_by: String,
    /// Hidden physical faults active when the lab starts.
    #[serde(default)]
    pub faults: Vec<FaultInjection>,
    /// The PLC program initially running in the CPU, if it differs from the project.
    #[serde(default)]
    pub loaded_program: Option<String>,
    pub diagnosis_options: Vec<DiagnosisOption>,
    /// Option IDs that identify the actual root cause.
    pub root_causes: Vec<Identifier>,
    /// Physical repairs offered to the student.
    #[serde(default)]
    pub repairs: Vec<RepairOption>,
    /// Criterion IDs that verify the repaired system.
    pub verification: Vec<Identifier>,
    #[serde(default)]
    pub knowledge_refs: Vec<Identifier>,
    /// Explanation revealed only after the repaired system passes verification.
    #[serde(default)]
    pub debrief: Option<Debrief>,
}

/// What a competent investigation should have found, shown after completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Debrief {
    pub root_cause: String,
    pub evidence: Vec<String>,
    pub repair: String,
    pub reasoning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosisOption {
    pub id: Identifier,
    pub label: String,
}

/// A physical repair. Applying it clears the listed faults if they are active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepairOption {
    pub id: Identifier,
    pub label: String,
    pub clears: Vec<Identifier>,
}
