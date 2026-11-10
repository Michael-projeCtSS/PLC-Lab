//! Ladder Diagram frontend (ADR-0050). A LAD program is a rung/grid document, not a node
//! graph: each rung is a series/parallel network of contacts that drives one or more
//! coils. The frontend validates the document and lowers it into the canonical PLC IR,
//! which the one PLC runtime executes like a program written in Structured Text.
//!
//! Lowering, per rung and per coil, in document order:
//! - a normally open contact reads its variable, a normally closed one negates it;
//! - a series is the AND of its items (an empty series at the rung root is TRUE: the
//!   left rail connected straight to the coils);
//! - a parallel block is the OR of its branches;
//! - an output coil assigns the network's result to its variable.
//!
//! Every rung, network element and coil keeps its persistent element id as its source
//! reference, so diagnostics and runtime faults point at the graphical element.
use plc_ir::{Expression, Program, Statement};
use plc_types::{
    PlcType, PlcValue,
    compatibility::{BinaryOperator, UnaryOperator},
    diagnostics::{Diagnostic, DiagnosticCategory, DiagnosticCode, Severity},
    identity::{DocumentId, PouId, SourceElementId, SymbolId},
    source::{SourceLanguage, SourceLocation, SourceMap, SourceRef},
    symbols::{Pou, PouKind, Scope, Symbol, SymbolTable, VariableCategory},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The `format` of a PLC Lab ladder document.
pub const FORMAT: &str = "plclab-lad";
/// The document version this frontend reads.
pub const VERSION: u32 = 1;
/// Upper bounds that keep validation and lowering cheap for any document.
pub const MAX_BYTES: usize = 262_144;
pub const MAX_RUNGS: usize = 256;
pub const MAX_ELEMENTS: usize = 4_096;
pub const MAX_VARIABLES: usize = 256;
pub const MAX_DEPTH: usize = 24;

/// Diagnostic codes of the LAD frontend.
pub mod codes {
    /// Not a PLC Lab ladder document, or one of an unknown version.
    pub const DOCUMENT: &str = "PLC-LAD-001";
    /// A variable declaration or the program name is invalid.
    pub const DECLARATION: &str = "PLC-LAD-002";
    /// A contact or coil names a variable that is not declared.
    pub const UNKNOWN: &str = "PLC-LAD-003";
    /// A coil writes an input.
    pub const TARGET: &str = "PLC-LAD-004";
    /// A rung or network is malformed.
    pub const RUNG: &str = "PLC-LAD-005";
    /// A size or nesting limit is exceeded.
    pub const LIMIT: &str = "PLC-LAD-006";
}

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
    /// The PLC type; version 1 supports BOOL.
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
    Series {
        id: u32,
        items: Vec<Node>,
    },
    Parallel {
        id: u32,
        branches: Vec<Node>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContactKind {
    NormallyOpen,
    NormallyClosed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Coil {
    pub id: u32,
    pub variable: String,
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

fn diagnostic(code: &'static str, message: String, element: Option<u32>) -> Diagnostic {
    let location = element
        .and_then(|id| SourceElementId::new(id).ok())
        .map(|id| SourceLocation::new(SourceRef::new(document(), id), SourceLanguage::Lad, None));
    Diagnostic::new(
        DiagnosticCode::new(code).expect("fixed LAD code"),
        DiagnosticCategory::CompilerDiagnostic,
        Severity::Error,
        message,
    )
    .expect("non-empty LAD message")
    .with_source(location)
}

fn document() -> DocumentId {
    DocumentId::new(1).expect("the main program is document 1")
}

/// Parse, validate and lower a ladder document.
pub fn compile_lad(source: &str) -> Result<CompiledLad, Vec<Diagnostic>> {
    if source.len() > MAX_BYTES {
        return Err(vec![diagnostic(
            codes::LIMIT,
            format!("A ladder document is limited to {MAX_BYTES} bytes."),
            None,
        )]);
    }
    let document: LadDocument = serde_json::from_str(source).map_err(|e| {
        vec![diagnostic(
            codes::DOCUMENT,
            format!("The ladder document is not valid: {e}."),
            None,
        )]
    })?;
    if document.format != FORMAT || document.version != VERSION {
        return Err(vec![diagnostic(
            codes::DOCUMENT,
            format!(
                "Expected a {FORMAT} document of version {VERSION}, found {} version {}.",
                document.format, document.version
            ),
            None,
        )]);
    }
    Lowering::new(&document)?.lower(document)
}

struct Lowering {
    owner: PouId,
    symbols: SymbolTable,
    sources: SourceMap,
    errors: Vec<Diagnostic>,
}

impl Lowering {
    fn new(document: &LadDocument) -> Result<Self, Vec<Diagnostic>> {
        let owner = PouId::new(1).expect("fixed entry identity");
        let mut symbols = SymbolTable::new();
        let pou = Pou::new(owner, document.name.clone(), PouKind::Program).map_err(|_| {
            vec![diagnostic(
                codes::DECLARATION,
                format!(
                    "The program name '{}' is not a valid identifier.",
                    document.name
                ),
                None,
            )]
        })?;
        symbols.declare_pou(pou).map_err(|e| vec![e.diagnostic()])?;
        let mut sources = SourceMap::new();
        sources
            .register_document(self::document(), SourceLanguage::Lad)
            .map_err(|_| {
                vec![diagnostic(
                    codes::DOCUMENT,
                    "The ladder document could not be registered.".into(),
                    None,
                )]
            })?;
        Ok(Self {
            owner,
            symbols,
            sources,
            errors: Vec::new(),
        })
    }

    fn lower(mut self, document: LadDocument) -> Result<CompiledLad, Vec<Diagnostic>> {
        self.declare(&document.variables);
        self.register(&document.rungs);
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        let mut body = Vec::new();
        for (index, rung) in document.rungs.iter().enumerate() {
            let number = index + 1;
            if rung.coils.is_empty() {
                self.error(
                    codes::RUNG,
                    format!("Rung {number} has no coil: a rung must drive at least one coil."),
                    Some(rung.id),
                );
                continue;
            }
            let Some(network) = self.network(&rung.network, number, true) else {
                continue;
            };
            for coil in &rung.coils {
                if let Some(target) = self.coil(coil, number) {
                    body.push(Statement::assign(self.at(coil.id), target, network.clone()));
                }
            }
        }
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        let program = Program::new(self.symbols, self.sources, self.owner, body);
        plc_ir::validate(&program)?;
        Ok(CompiledLad {
            program,
            name: document.name,
            variables: document.variables,
        })
    }

    fn error(&mut self, code: &'static str, message: String, element: Option<u32>) {
        self.errors.push(diagnostic(code, message, element));
    }

    fn at(&self, id: u32) -> SourceRef {
        SourceRef::new(
            document(),
            SourceElementId::new(id).expect("registered element ids are nonzero"),
        )
    }

    fn declare(&mut self, variables: &[LadVariable]) {
        if variables.len() > MAX_VARIABLES {
            self.error(
                codes::LIMIT,
                format!("A ladder program may declare at most {MAX_VARIABLES} variables."),
                None,
            );
            return;
        }
        for (index, variable) in variables.iter().enumerate() {
            if variable.data_type != "BOOL" {
                self.error(
                    codes::DECLARATION,
                    format!(
                        "Variable '{}' is {}; ladder programs use BOOL variables in this version.",
                        variable.name, variable.data_type
                    ),
                    None,
                );
                continue;
            }
            let category = match variable.direction {
                Direction::Input => VariableCategory::Input,
                Direction::Output => VariableCategory::Output,
                Direction::Local => VariableCategory::Local,
            };
            let id = SymbolId::new(u32::try_from(index + 1).unwrap_or(u32::MAX))
                .expect("symbol ids start at 1");
            let declared = Symbol::new(
                id,
                variable.name.clone(),
                PlcType::Bool,
                Scope::Pou(self.owner),
                category,
                None,
            )
            .and_then(|symbol| self.symbols.declare(symbol));
            if declared.is_err() {
                self.error(
                    codes::DECLARATION,
                    format!(
                        "Variable '{}' is not a valid name or is declared twice.",
                        variable.name
                    ),
                    None,
                );
            }
        }
    }

    /// Every rung, network element and coil id is a persistent source element: unique,
    /// nonzero and within the size limits.
    fn register(&mut self, rungs: &[Rung]) {
        if rungs.len() > MAX_RUNGS {
            self.error(
                codes::LIMIT,
                format!("A ladder program may have at most {MAX_RUNGS} rungs."),
                None,
            );
            return;
        }
        let mut ids = Vec::new();
        for rung in rungs {
            ids.push(rung.id);
            if !collect(&rung.network, &mut ids, 1) {
                self.error(
                    codes::LIMIT,
                    format!("A ladder network may nest at most {MAX_DEPTH} levels."),
                    Some(rung.id),
                );
                return;
            }
            ids.extend(rung.coils.iter().map(|c| c.id));
        }
        if ids.len() > MAX_ELEMENTS {
            self.error(
                codes::LIMIT,
                format!("A ladder program may have at most {MAX_ELEMENTS} elements."),
                None,
            );
            return;
        }
        let mut seen = BTreeSet::new();
        for id in ids {
            let Ok(element) = SourceElementId::new(id) else {
                self.error(
                    codes::RUNG,
                    "Element ids must be positive numbers.".into(),
                    None,
                );
                continue;
            };
            if !seen.insert(id) {
                self.error(
                    codes::RUNG,
                    format!("Element id {id} is used twice."),
                    Some(id),
                );
                continue;
            }
            let location = SourceLocation::new(
                SourceRef::new(document(), element),
                SourceLanguage::Lad,
                None,
            );
            if self.sources.register_element(location).is_err() {
                self.error(
                    codes::RUNG,
                    format!("Element id {id} could not be registered."),
                    Some(id),
                );
            }
        }
    }

    /// The declared variable `name` (case-insensitive, like Structured Text).
    fn variable(
        &mut self,
        name: &str,
        number: usize,
        element: u32,
    ) -> Option<(SymbolId, String, VariableCategory)> {
        let found = self
            .symbols
            .resolve(Scope::Pou(self.owner), name)
            .ok()
            .map(|s| (s.id(), s.name().to_owned(), s.category()));
        match found {
            Some(symbol) => Some(symbol),
            None => {
                self.error(
                    codes::UNKNOWN,
                    if name.trim().is_empty() {
                        format!("Rung {number}: an element has no variable.")
                    } else {
                        format!("Rung {number}: variable '{name}' is not declared.")
                    },
                    Some(element),
                );
                None
            }
        }
    }

    /// The power flowing out of `node` when power flows into it.
    fn network(&mut self, node: &Node, number: usize, root: bool) -> Option<Expression> {
        match node {
            Node::Contact {
                id,
                contact,
                variable,
            } => {
                let at = self.at(*id);
                let (symbol, _, _) = self.variable(variable, number, *id)?;
                let read = Expression::read(at, symbol, PlcType::Bool);
                Some(match contact {
                    ContactKind::NormallyOpen => read,
                    ContactKind::NormallyClosed => {
                        Expression::unary(at, PlcType::Bool, UnaryOperator::Not, read)
                    }
                })
            }
            Node::Series { id, items } => {
                if items.is_empty() && !root {
                    self.error(
                        codes::RUNG,
                        format!(
                            "Rung {number}: an empty series is only allowed as the whole network."
                        ),
                        Some(*id),
                    );
                    return None;
                }
                self.combine(*id, items, BinaryOperator::And, number)
            }
            Node::Parallel { id, branches } => {
                if branches.len() < 2 {
                    self.error(
                        codes::RUNG,
                        format!("Rung {number}: a parallel block needs at least two branches."),
                        Some(*id),
                    );
                    return None;
                }
                self.combine(*id, branches, BinaryOperator::Or, number)
            }
        }
    }

    fn combine(
        &mut self,
        id: u32,
        nodes: &[Node],
        operator: BinaryOperator,
        number: usize,
    ) -> Option<Expression> {
        let at = self.at(id);
        let mut operands = Vec::with_capacity(nodes.len());
        let mut complete = true;
        for node in nodes {
            match self.network(node, number, false) {
                Some(e) => operands.push(e),
                None => complete = false,
            }
        }
        if !complete {
            return None;
        }
        Some(match operands.len() {
            0 => Expression::constant(at, PlcValue::Bool(true)),
            1 => operands.pop().expect("one operand"),
            _ => Expression::logical(at, operator, operands),
        })
    }

    fn coil(&mut self, coil: &Coil, number: usize) -> Option<SymbolId> {
        let (id, name, category) = self.variable(&coil.variable, number, coil.id)?;
        if category == VariableCategory::Input {
            self.error(
                codes::TARGET,
                format!(
                    "Rung {number}: a coil cannot write input '{name}'; inputs come from the process image."
                ),
                Some(coil.id),
            );
            return None;
        }
        Some(id)
    }
}

/// Collect the element ids of `node`; `false` when it nests deeper than [`MAX_DEPTH`].
fn collect(node: &Node, ids: &mut Vec<u32>, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    match node {
        Node::Contact { id, .. } => {
            ids.push(*id);
            true
        }
        Node::Series { id, items: nodes }
        | Node::Parallel {
            id,
            branches: nodes,
        } => {
            ids.push(*id);
            nodes.iter().all(|n| collect(n, ids, depth + 1))
        }
    }
}
