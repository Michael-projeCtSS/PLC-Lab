//! Validation and lowering of a ladder document into the canonical IR (ADR-0051).
//!
//! Scan order, which the runtime executes like any other program:
//! 1. rungs top to bottom;
//! 2. in a rung, the network's elements depth-first, left to right; each element assigns
//!    the power leaving it to its observation symbol `LAD#<id>`; a box calls its block
//!    first; an edge contact updates its memory `EDGE#<id>` right after its own power;
//! 3. then the rung's coils in order, all reading the rung's power `LAD#<root id>`,
//!    which was computed once for the rung.
//!
//! A SET or RESET coil is a conditional assignment, so when several coils of one scan
//! write a variable, the one executed last wins.
use crate::{
    LadDocument, MAX_DEPTH, MAX_ELEMENTS, MAX_INSTANCES, MAX_RUNGS, MAX_VARIABLES,
    model::{BlockKind, Coil, CoilKind, CompareOp, ContactKind, Direction, Node, Operand},
};
use crate::{codes, literal};
use plc_ir::{CallBinding, Expression, FunctionBlockCall, Program, Statement};
use plc_types::{
    PlcType, PlcValue,
    compatibility::{BinaryOperator, UnaryOperator, binary_result},
    diagnostics::{Diagnostic, DiagnosticCategory, DiagnosticCode, Severity},
    function_blocks::{
        FunctionBlockCatalog, FunctionBlockDefinition, FunctionBlockInstance, StandardFunctionBlock,
    },
    identity::{
        DocumentId, FunctionBlockId, FunctionBlockInstanceId, ParameterId, PouId, SourceElementId,
        SymbolId,
    },
    source::{SourceLanguage, SourceLocation, SourceMap, SourceRef},
    symbols::{Pou, PouKind, Scope, Symbol, SymbolTable, VariableCategory},
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn document() -> DocumentId {
    DocumentId::new(1).expect("the main program is document 1")
}

pub(crate) fn diagnostic(code: &'static str, message: String, element: Option<u32>) -> Diagnostic {
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

/// A declared variable: its symbol, type and category.
#[derive(Clone, Copy)]
struct Var {
    id: SymbolId,
    kind: PlcType,
    category: VariableCategory,
}

pub(crate) struct Lowering {
    owner: PouId,
    symbols: SymbolTable,
    sources: SourceMap,
    catalog: FunctionBlockCatalog,
    definitions: BTreeMap<StandardFunctionBlock, FunctionBlockId>,
    instances: BTreeMap<String, u32>,
    next_symbol: u32,
    next_instance: u32,
    errors: Vec<Diagnostic>,
    body: Vec<Statement>,
    /// The rung being lowered, 1-based, for messages.
    rung: usize,
}

fn type_of(name: &str) -> Option<PlcType> {
    match name {
        "BOOL" => Some(PlcType::Bool),
        "INT" => Some(PlcType::Int),
        "REAL" => Some(PlcType::Real),
        "TIME" => Some(PlcType::Time),
        _ => None,
    }
}
fn type_name(kind: PlcType) -> &'static str {
    match kind {
        PlcType::Bool => "BOOL",
        PlcType::Int => "INT",
        PlcType::Real => "REAL",
        PlcType::Time => "TIME",
    }
}
fn standard(kind: BlockKind) -> StandardFunctionBlock {
    match kind {
        BlockKind::Ton => StandardFunctionBlock::Ton,
        BlockKind::Tof => StandardFunctionBlock::Tof,
        BlockKind::Tp => StandardFunctionBlock::Tp,
        BlockKind::Ctu => StandardFunctionBlock::Ctu,
        BlockKind::Ctd => StandardFunctionBlock::Ctd,
    }
}
fn operator(op: CompareOp) -> BinaryOperator {
    match op {
        CompareOp::Eq => BinaryOperator::Equal,
        CompareOp::Ne => BinaryOperator::NotEqual,
        CompareOp::Gt => BinaryOperator::Greater,
        CompareOp::Ge => BinaryOperator::GreaterEqual,
        CompareOp::Lt => BinaryOperator::Less,
        CompareOp::Le => BinaryOperator::LessEqual,
    }
}

impl Lowering {
    pub(crate) fn new(document: &LadDocument) -> Result<Self, Vec<Diagnostic>> {
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
            catalog: FunctionBlockCatalog::new(),
            definitions: BTreeMap::new(),
            instances: BTreeMap::new(),
            next_symbol: 1,
            next_instance: 1,
            errors: Vec::new(),
            body: Vec::new(),
            rung: 0,
        })
    }

    pub(crate) fn lower(mut self, document: &LadDocument) -> Result<Program, Vec<Diagnostic>> {
        self.declare(document);
        self.register(document);
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        for (index, rung) in document.rungs.iter().enumerate() {
            self.rung = index + 1;
            if rung.coils.is_empty() {
                let message = format!("Rung {}: a rung must drive at least one coil.", self.rung);
                self.error(codes::RUNG, message, Some(rung.id));
                continue;
            }
            let Some(power) = self.node(&rung.network, None, true) else {
                continue;
            };
            for coil in &rung.coils {
                self.coil(coil, &power);
            }
        }
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        let program = Program::new(self.symbols, self.sources, self.owner, self.body)
            .with_function_blocks(self.catalog);
        plc_ir::validate(&program)?;
        Ok(program)
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
    fn next_symbol(&mut self) -> SymbolId {
        let id = SymbolId::new(self.next_symbol).expect("symbol ids start at 1");
        self.next_symbol += 1;
        id
    }

    fn declare(&mut self, document: &LadDocument) {
        if document.variables.len() > MAX_VARIABLES {
            self.error(
                codes::LIMIT,
                format!("A ladder program may declare at most {MAX_VARIABLES} variables."),
                None,
            );
            return;
        }
        for variable in &document.variables {
            let Some(kind) = type_of(&variable.data_type) else {
                self.error(
                    codes::DECLARATION,
                    format!(
                        "Variable '{}' has type {}; ladder programs use BOOL, INT, REAL or TIME.",
                        variable.name, variable.data_type
                    ),
                    None,
                );
                continue;
            };
            let category = match variable.direction {
                Direction::Input => VariableCategory::Input,
                Direction::Output => VariableCategory::Output,
                Direction::Local => VariableCategory::Local,
            };
            let reserved = !crate::is_identifier(&variable.name);
            let id = self.next_symbol();
            let declared = Symbol::new(
                id,
                variable.name.clone(),
                kind,
                Scope::Pou(self.owner),
                category,
                None,
            )
            .and_then(|symbol| self.symbols.declare(symbol));
            if reserved || declared.is_err() {
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
    fn register(&mut self, doc: &LadDocument) {
        if doc.rungs.len() > MAX_RUNGS {
            self.error(
                codes::LIMIT,
                format!("A ladder program may have at most {MAX_RUNGS} rungs."),
                None,
            );
            return;
        }
        let mut ids = Vec::new();
        for rung in &doc.rungs {
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
    fn variable(&mut self, name: &str, element: u32) -> Option<Var> {
        let found = self
            .symbols
            .resolve(Scope::Pou(self.owner), name)
            .ok()
            .filter(|s| !crate::is_synthetic(s.name()))
            .map(|s| Var {
                id: s.id(),
                kind: s.plc_type(),
                category: s.category(),
            });
        if found.is_none() {
            let rung = self.rung;
            self.error(
                codes::UNKNOWN,
                if name.trim().is_empty() {
                    format!("Rung {rung}: an element has no variable.")
                } else {
                    format!("Rung {rung}: variable '{name}' is not declared.")
                },
                Some(element),
            );
        }
        found
    }

    /// A BOOL variable for a contact or coil.
    fn boolean(&mut self, name: &str, element: u32) -> Option<Var> {
        let var = self.variable(name, element)?;
        if var.kind != PlcType::Bool {
            let rung = self.rung;
            self.error(
                codes::TYPE,
                format!(
                    "Rung {rung}: '{name}' is {}; contacts and coils use BOOL variables.",
                    type_name(var.kind)
                ),
                Some(element),
            );
            return None;
        }
        Some(var)
    }

    /// A hidden BOOL symbol the compiler owns (`LAD#<id>`, `EDGE#<id>`).
    fn synthetic(&mut self, prefix: &str, element: u32) -> SymbolId {
        let id = self.next_symbol();
        let symbol = Symbol::new(
            id,
            format!("{prefix}#{element}"),
            PlcType::Bool,
            Scope::Pou(self.owner),
            VariableCategory::Local,
            Some(self.at(element)),
        )
        .expect("generated names are valid paths");
        self.symbols
            .declare(symbol)
            .expect("element ids are unique, so generated names are");
        id
    }

    /// `power AND condition`, without the AND when the power is the left rail.
    fn and(&self, at: SourceRef, power: Option<&Expression>, condition: Expression) -> Expression {
        match power {
            None => condition,
            Some(p) => Expression::logical(at, BinaryOperator::And, vec![p.clone(), condition]),
        }
    }

    /// Lower `node` with `power` flowing into it (`None`: the left rail) and record the
    /// power leaving it in its observation symbol; the read of that symbol is returned.
    fn node(&mut self, node: &Node, power: Option<&Expression>, root: bool) -> Option<Expression> {
        let id = node.id();
        let at = self.at(id);
        let out = match node {
            Node::Contact {
                contact, variable, ..
            } => {
                let var = self.boolean(variable, id)?;
                let read = Expression::read(at, var.id, PlcType::Bool);
                match contact {
                    ContactKind::NormallyOpen => self.and(at, power, read),
                    ContactKind::NormallyClosed => self.and(
                        at,
                        power,
                        Expression::unary(at, PlcType::Bool, UnaryOperator::Not, read),
                    ),
                    ContactKind::PositiveEdge | ContactKind::NegativeEdge => {
                        let memory = self.synthetic("EDGE", id);
                        let before = Expression::read(at, memory, PlcType::Bool);
                        let not = |e: Expression| {
                            Expression::unary(at, PlcType::Bool, UnaryOperator::Not, e)
                        };
                        let edge = if *contact == ContactKind::PositiveEdge {
                            Expression::logical(
                                at,
                                BinaryOperator::And,
                                vec![read.clone(), not(before)],
                            )
                        } else {
                            Expression::logical(
                                at,
                                BinaryOperator::And,
                                vec![not(read.clone()), before],
                            )
                        };
                        let out = self.and(at, power, edge);
                        let symbol = self.synthetic("LAD", id);
                        self.body.push(Statement::assign(at, symbol, out));
                        // The memory follows the variable every scan, powered or not.
                        self.body.push(Statement::assign(at, memory, read));
                        return Some(Expression::read(at, symbol, PlcType::Bool));
                    }
                }
            }
            Node::Compare {
                op, left, right, ..
            } => {
                let condition = self.compare(*op, left, right, id)?;
                self.and(at, power, condition)
            }
            Node::Block {
                block,
                instance,
                preset,
                reset,
                output,
                ..
            } => self.block(
                *block,
                instance,
                preset.as_ref(),
                reset.as_ref(),
                output.as_deref(),
                power,
                id,
            )?,
            Node::Series { items, .. } => {
                if items.is_empty() && !root {
                    let rung = self.rung;
                    self.error(
                        codes::RUNG,
                        format!(
                            "Rung {rung}: an empty series is only allowed as the whole network."
                        ),
                        Some(id),
                    );
                    return None;
                }
                let mut current = power.cloned();
                let mut complete = true;
                for item in items {
                    match self.node(item, current.as_ref(), false) {
                        Some(out) => current = Some(out),
                        None => complete = false,
                    }
                }
                if !complete {
                    return None;
                }
                current.unwrap_or_else(|| Expression::constant(at, PlcValue::Bool(true)))
            }
            Node::Parallel { branches, .. } => {
                if branches.len() < 2 {
                    let rung = self.rung;
                    self.error(
                        codes::RUNG,
                        format!("Rung {rung}: a parallel block needs at least two branches."),
                        Some(id),
                    );
                    return None;
                }
                let mut outs = Vec::with_capacity(branches.len());
                let mut complete = true;
                for branch in branches {
                    match self.node(branch, power, false) {
                        Some(out) => outs.push(out),
                        None => complete = false,
                    }
                }
                if !complete {
                    return None;
                }
                Expression::logical(at, BinaryOperator::Or, outs)
            }
        };
        let symbol = self.synthetic("LAD", id);
        self.body.push(Statement::assign(at, symbol, out));
        Some(Expression::read(at, symbol, PlcType::Bool))
    }

    /// A typed operand expression, or `None` with a diagnostic.
    fn operand(
        &mut self,
        operand: &Operand,
        kind: PlcType,
        element: u32,
        what: &str,
    ) -> Option<Expression> {
        let at = self.at(element);
        let rung = self.rung;
        match operand {
            Operand::Variable(name) => {
                let var = self.variable(name, element)?;
                if var.kind != kind {
                    self.error(
                        codes::TYPE,
                        format!(
                            "Rung {rung}: {what} needs {}, but '{name}' is {}.",
                            type_name(kind),
                            type_name(var.kind)
                        ),
                        Some(element),
                    );
                    return None;
                }
                Some(Expression::read(at, var.id, kind))
            }
            Operand::Constant(text) => match literal::parse(text, kind) {
                Ok(value) => Some(Expression::constant(at, value)),
                Err(reason) => {
                    self.error(
                        codes::TYPE,
                        format!("Rung {rung}: {what}: {reason}"),
                        Some(element),
                    );
                    None
                }
            },
        }
    }

    fn operand_type(&mut self, operand: &Operand) -> Option<PlcType> {
        match operand {
            Operand::Variable(name) => self
                .symbols
                .resolve(Scope::Pou(self.owner), name)
                .ok()
                .map(|s| s.plc_type()),
            Operand::Constant(_) => None,
        }
    }

    fn compare(
        &mut self,
        op: CompareOp,
        left: &Operand,
        right: &Operand,
        element: u32,
    ) -> Option<Expression> {
        let rung = self.rung;
        // The variable side decides the type; a literal takes the same type.
        let kind = match (self.operand_type(left), self.operand_type(right)) {
            (Some(a), Some(b)) if a != b => {
                self.error(
                    codes::TYPE,
                    format!(
                        "Rung {rung}: a comparison needs two values of one type, not {} and {}.",
                        type_name(a),
                        type_name(b)
                    ),
                    Some(element),
                );
                return None;
            }
            (Some(a), _) | (None, Some(a)) => a,
            (None, None) => {
                // Unknown variables are reported below; two literals have no type.
                if matches!(left, Operand::Constant(_)) && matches!(right, Operand::Constant(_)) {
                    self.error(
                        codes::TYPE,
                        format!("Rung {rung}: a comparison needs at least one variable."),
                        Some(element),
                    );
                    return None;
                }
                let name = match (left, right) {
                    (Operand::Variable(n), _) | (_, Operand::Variable(n)) => n.clone(),
                    _ => String::new(),
                };
                self.variable(&name, element);
                return None;
            }
        };
        if binary_result(operator(op), kind, kind).is_none() {
            self.error(
                codes::TYPE,
                format!(
                    "Rung {rung}: {} cannot be compared with GT, GE, LT or LE; use EQ or NE.",
                    type_name(kind)
                ),
                Some(element),
            );
            return None;
        }
        let a = self.operand(left, kind, element, "the comparison");
        let b = self.operand(right, kind, element, "the comparison");
        Some(Expression::binary(
            self.at(element),
            PlcType::Bool,
            operator(op),
            a?,
            b?,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn block(
        &mut self,
        kind: BlockKind,
        instance: &str,
        preset: Option<&Operand>,
        reset: Option<&Operand>,
        output: Option<&str>,
        power: Option<&Expression>,
        element: u32,
    ) -> Option<Expression> {
        let at = self.at(element);
        let rung = self.rung;
        let block = standard(kind);
        let timer = matches!(kind, BlockKind::Ton | BlockKind::Tof | BlockKind::Tp);
        let name = block.name();
        let mut valid = true;
        // The instance: a fresh, valid name that no variable uses.
        let upper = instance.to_ascii_uppercase();
        if self
            .symbols
            .resolve(Scope::Pou(self.owner), instance)
            .is_ok()
            || crate::is_synthetic(instance)
        {
            self.error(
                codes::BLOCK,
                format!("Rung {rung}: instance '{instance}' has the name of a variable."),
                Some(element),
            );
            valid = false;
        } else if let Some(first) = self.instances.get(&upper) {
            let first = *first;
            self.error(
                codes::BLOCK,
                format!(
                    "Rung {rung}: instance '{instance}' is already used by another box (element {first})."
                ),
                Some(element),
            );
            valid = false;
        }
        if self.instances.len() >= MAX_INSTANCES {
            self.error(
                codes::LIMIT,
                format!(
                    "A ladder program may have at most {MAX_INSTANCES} timer and counter boxes."
                ),
                Some(element),
            );
            return None;
        }
        let preset_name = if timer { "PT" } else { "PV" };
        let preset_type = if timer { PlcType::Time } else { PlcType::Int };
        let preset = match preset {
            Some(p) => self.operand(p, preset_type, element, &format!("{name}.{preset_name}")),
            None => {
                self.error(
                    codes::BLOCK,
                    format!("Rung {rung}: {name} '{instance}' needs {preset_name}."),
                    Some(element),
                );
                None
            }
        };
        let reset_name = if kind == BlockKind::Ctd { "LD" } else { "R" };
        let reset = match (reset, timer) {
            (Some(_), true) => {
                self.error(
                    codes::BLOCK,
                    format!("Rung {rung}: {name} has no reset input."),
                    Some(element),
                );
                valid = false;
                None
            }
            (Some(r), false) => {
                let e = self.operand(r, PlcType::Bool, element, &format!("{name}.{reset_name}"));
                valid &= e.is_some();
                e
            }
            (None, _) => None,
        };
        let output_name = if timer { "ET" } else { "CV" };
        let output = match output {
            Some(target) => {
                let var = self.variable(target, element);
                match var {
                    Some(v) if v.category == VariableCategory::Input => {
                        self.error(
                            codes::TARGET,
                            format!(
                                "Rung {rung}: {name}.{output_name} cannot write input '{target}'."
                            ),
                            Some(element),
                        );
                        valid = false;
                        None
                    }
                    Some(v) if v.kind != preset_type => {
                        self.error(
                            codes::TYPE,
                            format!(
                                "Rung {rung}: {name}.{output_name} is {}, but '{target}' is {}.",
                                type_name(preset_type),
                                type_name(v.kind)
                            ),
                            Some(element),
                        );
                        valid = false;
                        None
                    }
                    Some(v) => Some(v.id),
                    None => {
                        valid = false;
                        None
                    }
                }
            }
            None => None,
        };
        let preset = preset?;
        if !valid || !crate::is_identifier(instance) {
            if valid {
                self.error(
                    codes::BLOCK,
                    format!("Rung {rung}: '{instance}' is not a valid instance name."),
                    Some(element),
                );
            }
            return None;
        }
        let definition = match self.definitions.get(&block) {
            Some(id) => *id,
            None => {
                let id = FunctionBlockId::new(
                    u32::try_from(self.definitions.len() + 1).unwrap_or(u32::MAX),
                )
                .expect("nonzero definition identity");
                self.catalog
                    .declare_definition(FunctionBlockDefinition::standard(id, block))
                    .expect("one definition per standard block");
                self.definitions.insert(block, id);
                id
            }
        };
        let id = FunctionBlockInstanceId::new(self.next_instance).expect("bounded instances");
        self.next_instance += 1;
        if FunctionBlockInstance::new(id, definition, self.owner, instance, Some(at))
            .and_then(|i| self.catalog.declare_instance(i))
            .is_err()
        {
            self.error(
                codes::BLOCK,
                format!("Rung {rung}: '{instance}' is not a valid instance name."),
                Some(element),
            );
            return None;
        }
        self.instances.insert(upper, element);
        let parameter = |pin: &str| {
            let p = block.parameter_named(pin).expect("standard pin");
            ParameterId::new(p.id).expect("nonzero parameter id")
        };
        let input = match kind {
            BlockKind::Ctu => "CU",
            BlockKind::Ctd => "CD",
            _ => "IN",
        };
        let flowing = power
            .cloned()
            .unwrap_or_else(|| Expression::constant(at, PlcValue::Bool(true)));
        let mut bindings = vec![
            CallBinding::input(parameter(input), flowing),
            CallBinding::input(parameter(preset_name), preset),
        ];
        if let Some(r) = reset {
            bindings.push(CallBinding::input(parameter(reset_name), r));
        }
        if let Some(target) = output {
            bindings.push(CallBinding::output(parameter(output_name), target));
        }
        let call = FunctionBlockCall::new(id, bindings).expect("distinct pins");
        self.body.push(Statement::function_block_call(at, call));
        Some(Expression::function_block_output(
            at,
            PlcType::Bool,
            id,
            parameter("Q"),
        ))
    }

    fn coil(&mut self, coil: &Coil, power: &Expression) {
        let at = self.at(coil.id);
        let Some(var) = self.boolean(&coil.variable, coil.id) else {
            return;
        };
        if var.category == VariableCategory::Input {
            let rung = self.rung;
            let name = coil.variable.clone();
            self.error(
                codes::TARGET,
                format!(
                    "Rung {rung}: a coil cannot write input '{name}'; inputs come from the process image."
                ),
                Some(coil.id),
            );
            return;
        }
        let statement = match coil.kind {
            CoilKind::Output => Statement::assign(at, var.id, power.clone()),
            CoilKind::Set | CoilKind::Reset => Statement::conditional(
                at,
                power.clone(),
                vec![Statement::assign(
                    at,
                    var.id,
                    Expression::constant(at, PlcValue::Bool(coil.kind == CoilKind::Set)),
                )],
                Vec::new(),
            ),
        };
        self.body.push(statement);
    }
}

/// Collect the element ids of `node`; `false` when it nests deeper than [`MAX_DEPTH`].
fn collect(node: &Node, ids: &mut Vec<u32>, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    ids.push(node.id());
    match node {
        Node::Series { items: nodes, .. }
        | Node::Parallel {
            branches: nodes, ..
        } => nodes.iter().all(|n| collect(n, ids, depth + 1)),
        _ => true,
    }
}
