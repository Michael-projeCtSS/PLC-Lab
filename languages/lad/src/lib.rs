//! Ladder Diagram frontend (ADR-0050, ADR-0051). A LAD program is a rung/grid document,
//! not a node graph: each rung is a series/parallel network of contacts, comparators and
//! timer/counter boxes that drives coils. The frontend validates the document and lowers
//! it into the canonical PLC IR, which the one PLC runtime executes like a program written
//! in Structured Text; timers and counters are the runtime's standard blocks.
//!
//! Every rung, network element and coil keeps its persistent element id as its source
//! reference, so diagnostics and runtime faults point at the graphical element. The power
//! leaving each network element is computed by the runtime into a compiler-owned symbol
//! `LAD#<id>`, which online monitoring reads (see [`lower`] for the scan order).
mod literal;
mod lower;
mod model;

pub use model::{
    BlockKind, Coil, CoilKind, CompareOp, ContactKind, Direction, LadDocument, LadVariable, Node,
    Operand, Rung,
};
use plc_ir::Program;
use plc_types::diagnostics::Diagnostic;

/// The `format` of a PLC Lab ladder document.
pub const FORMAT: &str = "plclab-lad";
/// The document version this frontend writes; it also reads version 1.
pub const VERSION: u32 = 2;
/// Upper bounds that keep validation and lowering cheap for any document.
pub const MAX_BYTES: usize = 262_144;
pub const MAX_RUNGS: usize = 256;
pub const MAX_ELEMENTS: usize = 4_096;
pub const MAX_VARIABLES: usize = 256;
pub const MAX_INSTANCES: usize = 128;
pub const MAX_DEPTH: usize = 24;

/// Diagnostic codes of the LAD frontend.
pub mod codes {
    /// Not a PLC Lab ladder document, or one of an unknown version.
    pub const DOCUMENT: &str = "PLC-LAD-001";
    /// A variable declaration or the program name is invalid.
    pub const DECLARATION: &str = "PLC-LAD-002";
    /// An element names a variable that is not declared.
    pub const UNKNOWN: &str = "PLC-LAD-003";
    /// A coil or block output writes an input.
    pub const TARGET: &str = "PLC-LAD-004";
    /// A rung or network is malformed.
    pub const RUNG: &str = "PLC-LAD-005";
    /// A size or nesting limit is exceeded.
    pub const LIMIT: &str = "PLC-LAD-006";
    /// A value of the wrong type: a non-BOOL contact or coil, a comparison of two types, a
    /// block parameter of the wrong type, an invalid literal.
    pub const TYPE: &str = "PLC-LAD-007";
    /// A timer or counter box is incomplete or its instance name is taken.
    pub const BLOCK: &str = "PLC-LAD-008";
}

/// A validated ladder program lowered into the canonical IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledLad {
    pub program: Program,
    pub name: String,
    pub variables: Vec<LadVariable>,
}

/// Whether `source` is a ladder document rather than Structured Text: a JSON object that
/// declares the ladder format. Structured Text cannot start with `{`.
pub fn is_lad(source: &str) -> bool {
    source.trim_start().starts_with('{') && source.contains(FORMAT)
}

/// Whether `name` is a symbol the compiler owns: the power leaving an element
/// (`LAD#<id>`) or an edge contact's memory (`EDGE#<id>`). Students cannot declare these
/// names, and variable lists hide them.
pub fn is_synthetic(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ["LAD#", "EDGE#"].iter().any(|prefix| {
        upper
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// Parse, validate and lower a ladder document.
pub fn compile_lad(source: &str) -> Result<CompiledLad, Vec<Diagnostic>> {
    if source.len() > MAX_BYTES {
        return Err(vec![lower::diagnostic(
            codes::LIMIT,
            format!("A ladder document is limited to {MAX_BYTES} bytes."),
            None,
        )]);
    }
    let document: LadDocument = serde_json::from_str(source).map_err(|e| {
        vec![lower::diagnostic(
            codes::DOCUMENT,
            format!("The ladder document is not valid: {e}."),
            None,
        )]
    })?;
    if document.format != FORMAT || !(1..=VERSION).contains(&document.version) {
        return Err(vec![lower::diagnostic(
            codes::DOCUMENT,
            format!(
                "Expected a {FORMAT} document of version 1 to {VERSION}, found {} version {}.",
                document.format, document.version
            ),
            None,
        )]);
    }
    let program = lower::Lowering::new(&document)?.lower(&document)?;
    Ok(CompiledLad {
        program,
        name: document.name,
        variables: document.variables,
    })
}

/// A name a student can declare: an IEC identifier, not a compiler-owned name.
pub(crate) fn is_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
