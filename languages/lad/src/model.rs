//! The ladder document (ADR-0050, ADR-0051): rungs of series/parallel networks that drive
//! coils. Version 2 adds edge contacts, comparators, timer and counter boxes, SET and
//! RESET coils and INT/REAL/TIME variables; every version 1 document is a valid version 2
//! document.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LadDocument {
    pub format: String,
    pub version: u32,
    /// The program name (`PROGRAM <name>` in Structured Text).
    pub name: String,
    pub variables: Vec<LadVariable>,
    pub rungs: Vec<Rung>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LadVariable {
    pub name: String,
    /// BOOL, INT, REAL or TIME.
    #[serde(rename = "type")]
    pub data_type: String,
    pub direction: Direction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Input,
    Output,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rung {
    pub id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub network: Node,
    pub coils: Vec<Coil>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Node {
    Contact {
        id: u32,
        contact: ContactKind,
        variable: String,
    },
    /// A comparison that passes power while it holds: `left op right`.
    Compare {
        id: u32,
        op: CompareOp,
        left: Operand,
        right: Operand,
    },
    /// A standard timer or counter box. Its first BOOL input (IN, CU or CD) is the power
    /// flowing into it; its Q output is the power flowing on.
    Block {
        id: u32,
        block: BlockKind,
        /// The instance name, like a `VAR` of the block's type in Structured Text.
        instance: String,
        /// PT of a timer (TIME), PV of a counter (INT).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preset: Option<Operand>,
        /// R of CTU, LD of CTD (BOOL); not used by timers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reset: Option<Operand>,
        /// The variable ET (timers, TIME) or CV (counters, INT) is written to.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    Series {
        id: u32,
        items: Vec<Node>,
    },
    Parallel {
        id: u32,
        branches: Vec<Node>,
    },
}

impl Node {
    pub fn id(&self) -> u32 {
        match self {
            Self::Contact { id, .. }
            | Self::Compare { id, .. }
            | Self::Block { id, .. }
            | Self::Series { id, .. }
            | Self::Parallel { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContactKind {
    NormallyOpen,
    NormallyClosed,
    /// Passes power in the one scan in which its variable changed from FALSE to TRUE.
    PositiveEdge,
    /// Passes power in the one scan in which its variable changed from TRUE to FALSE.
    NegativeEdge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BlockKind {
    Ton,
    Tof,
    Tp,
    Ctu,
    Ctd,
}

/// A block parameter or comparator operand: a declared variable or a literal in IEC
/// syntax (`TRUE`, `10`, `2.5`, `T#5s`), typed by where it is used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Operand {
    Variable(String),
    Constant(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Coil {
    pub id: u32,
    pub variable: String,
    #[serde(default, skip_serializing_if = "CoilKind::is_output")]
    pub kind: CoilKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CoilKind {
    /// The variable follows the rung's power: `X := power`.
    #[default]
    Output,
    /// Powered, the variable becomes TRUE; unpowered, it keeps its value.
    Set,
    /// Powered, the variable becomes FALSE; unpowered, it keeps its value.
    Reset,
}

impl CoilKind {
    fn is_output(&self) -> bool {
        *self == Self::Output
    }
}
