//! Checks data and types only. Never calculates a PLC expression's value.

use crate::{
    CallBinding, Expression, ExpressionKind, FunctionBlockCall, Program, Statement, StatementKind,
};
use plc_types::{
    PlcType,
    compatibility::{self, AssignmentCompatibility},
    diagnostics::{
        Diagnostic, DiagnosticCategory, DiagnosticCode, RelatedIdentity, Severity, codes,
    },
    function_blocks::{FunctionBlockKind, StandardFunctionBlock},
    identity::SymbolId,
    source::SourceRef,
    symbols::{PouKind, Scope, VariableCategory},
};

pub const MAX_VALIDATION_DEPTH: usize = 64;
pub const MAX_VALIDATION_NODES: usize = 10_000;

/// Cannot be constructed by callers. Its shared borrow prevents mutation after validation.
#[derive(Debug)]
pub struct ValidatedProgram<'a> {
    program: &'a Program,
}
impl<'a> ValidatedProgram<'a> {
    pub fn program(&self) -> &'a Program {
        self.program
    }
}

pub fn validate(program: &Program) -> Result<ValidatedProgram<'_>, Vec<Diagnostic>> {
    let mut validator = Validator {
        program,
        diagnostics: Vec::new(),
        nodes: 0,
        halted: false,
    };
    if !program
        .symbols()
        .pou(program.entry())
        .is_some_and(|pou| pou.kind() == PouKind::Program)
    {
        validator.emit(
            codes::INVALID_ENTRY,
            "The entry must identify a declared Program.",
            None,
            Some(RelatedIdentity::Pou(program.entry())),
        );
    }
    for symbol in program.symbols().symbols() {
        if validator.halted {
            break;
        }
        validator.visit(symbol.source(), 0);
    }
    validator.catalog();
    validator.statements(program.body(), 0);
    if validator.diagnostics.is_empty() {
        Ok(ValidatedProgram { program })
    } else {
        Err(validator.diagnostics)
    }
}

struct Validator<'a> {
    program: &'a Program,
    diagnostics: Vec<Diagnostic>,
    nodes: usize,
    halted: bool,
}
impl Validator<'_> {
    fn catalog(&mut self) {
        for definition in self.program.function_blocks().definitions() {
            if !self.visit(definition.source(), 0) {
                return;
            }
            for _ in definition.parameters() {
                if !self.visit(None, 0) {
                    return;
                }
            }
        }
        for instance in self.program.function_blocks().instances() {
            if !self.visit(instance.source(), 0) {
                return;
            }
            if self.program.symbols().pou(instance.owner()).is_none() {
                self.emit(
                    codes::INVALID_FB_OWNER,
                    "A function-block instance requires a declared POU owner.",
                    instance.source(),
                    Some(RelatedIdentity::FunctionBlockInstance(instance.id())),
                );
            }
            if self
                .program
                .function_blocks()
                .definition(instance.definition())
                .is_none()
            {
                self.emit(
                    codes::UNKNOWN_FB_DEFINITION,
                    "The instance refers to an undeclared function-block definition.",
                    instance.source(),
                    Some(RelatedIdentity::FunctionBlockDefinition(
                        instance.definition(),
                    )),
                );
            }
        }
    }
    fn call(&mut self, call: &FunctionBlockCall, source: SourceRef, depth: usize) {
        let related = Some(RelatedIdentity::FunctionBlockInstance(call.instance()));
        let Some(instance) = self.program.function_blocks().instance(call.instance()) else {
            self.emit(
                codes::UNKNOWN_FB_INSTANCE,
                "The called function-block instance is not declared.",
                Some(source),
                related,
            );
            return;
        };
        if instance.owner() != self.program.entry() {
            self.emit(
                codes::INVALID_FB_OWNER,
                "The entry program cannot call another POU's instance.",
                Some(source),
                related,
            );
            return;
        }
        let Some(definition) = self
            .program
            .function_blocks()
            .definition(instance.definition())
        else {
            self.emit(
                codes::UNKNOWN_FB_DEFINITION,
                "The called instance has no declared definition.",
                Some(source),
                related,
            );
            return;
        };
        if definition.kind() != FunctionBlockKind::Standard(StandardFunctionBlock::Ton) {
            self.emit(
                codes::UNSUPPORTED_FB_KIND,
                "This function-block kind is not executable in Alpha.",
                Some(source),
                related,
            );
            return;
        }
        let mut seen = std::collections::BTreeSet::new();
        for binding in call.bindings() {
            if !self.visit(Some(source), depth + 1) {
                return;
            }
            if !seen.insert(binding.parameter()) {
                self.emit(
                    codes::DUPLICATE_FB_BINDING,
                    "A function-block parameter is bound more than once.",
                    Some(source),
                    related,
                );
            }
            if definition.parameter(binding.parameter()).is_none() {
                self.emit(
                    codes::UNKNOWN_FB_PARAMETER,
                    format!(
                        "Parameter {} does not belong to the resolved definition.",
                        binding.parameter().get()
                    ),
                    Some(source),
                    related,
                );
            }
        }
        // Signature order is deterministic even when named associations arrive shuffled.
        for parameter in definition.parameters() {
            if self.halted {
                return;
            }
            let Some(binding) = call
                .bindings()
                .iter()
                .find(|b| b.parameter() == parameter.id())
            else {
                if parameter.required() {
                    self.emit(
                        codes::MISSING_FB_INPUT,
                        format!(
                            "Required function-block input {} is missing.",
                            parameter.name()
                        ),
                        Some(source),
                        related,
                    );
                }
                continue;
            };
            if binding.direction() != parameter.direction() {
                self.emit(
                    codes::FB_PARAMETER_DIRECTION,
                    format!(
                        "The association for {} has the wrong direction.",
                        parameter.name()
                    ),
                    Some(source),
                    related,
                );
                continue;
            }
            let actual = match binding {
                CallBinding::Input { value, .. } => self.expression(value, depth + 1),
                CallBinding::Output { target, .. } => self.write_type(*target, source),
                CallBinding::InOut { .. } => {
                    self.emit(
                        codes::FB_PARAMETER_DIRECTION,
                        "InOut associations are not executable for TON.",
                        Some(source),
                        related,
                    );
                    None
                }
            };
            if let Some(actual) = actual
                && actual != parameter.plc_type()
            {
                let affected = match binding {
                    CallBinding::Output { target, .. } => Some(RelatedIdentity::Symbol(*target)),
                    _ => related,
                };
                self.emit(
                    codes::FB_PARAMETER_TYPE,
                    format!(
                        "Parameter {} requires {:?}, received {actual:?}.",
                        parameter.name(),
                        parameter.plc_type()
                    ),
                    Some(source),
                    affected,
                );
            }
        }
    }
    fn emit(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        source: Option<SourceRef>,
        related: Option<RelatedIdentity>,
    ) {
        // All codes are tested constants and every producer supplies nonblank fixed text.
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::new(code).expect("core diagnostic code invariant"),
            DiagnosticCategory::CompilerDiagnostic,
            Severity::Error,
            message,
        )
        .expect("core diagnostic message invariant")
        .with_source(source.and_then(|s| self.program.sources().get(s)).cloned());
        if let Some(id) = related {
            diagnostic = diagnostic.with_related(id);
        }
        self.diagnostics.push(diagnostic);
    }
    fn visit(&mut self, source: Option<SourceRef>, depth: usize) -> bool {
        if self.halted {
            return false;
        }
        if depth > MAX_VALIDATION_DEPTH || self.nodes >= MAX_VALIDATION_NODES {
            self.emit(
                codes::VALIDATION_LIMIT,
                "The semantic representation exceeds the validation size or nesting limit.",
                source,
                None,
            );
            self.halted = true;
            return false;
        }
        self.nodes += 1;
        if let Some(source) = source
            && self.program.sources().get(source).is_none()
        {
            self.emit(
                codes::UNKNOWN_SOURCE,
                format!(
                    "Source element {} in document {} is not registered.",
                    source.element_id().get(),
                    source.document_id().get()
                ),
                None,
                None,
            );
        }
        true
    }
    fn symbol_type(&mut self, id: SymbolId, source: SourceRef) -> Option<PlcType> {
        match self
            .program
            .symbols()
            .visible(Scope::Pou(self.program.entry()), id)
        {
            Ok(symbol) => Some(symbol.plc_type()),
            Err(error) => {
                self.diagnostics.push(
                    error
                        .diagnostic()
                        .with_source(self.program.sources().get(source).cloned())
                        .with_related(RelatedIdentity::Symbol(id)),
                );
                None
            }
        }
    }
    fn expression(&mut self, expression: &Expression, depth: usize) -> Option<PlcType> {
        if !self.visit(Some(expression.source()), depth) {
            return None;
        }
        let inferred = match expression.kind() {
            ExpressionKind::Constant(value) => Some(value.plc_type()),
            ExpressionKind::Read(id) => self.symbol_type(*id, expression.source()),
            ExpressionKind::Unary { operator, operand } => {
                let operand = self.expression(operand, depth + 1)?;
                let result = compatibility::unary_result(*operator, operand);
                if result.is_none() {
                    self.emit(
                        codes::OPERAND_TYPE,
                        format!("{operator:?} does not accept {operand:?}."),
                        Some(expression.source()),
                        None,
                    );
                }
                result
            }
            ExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.expression(left, depth + 1);
                let right = self.expression(right, depth + 1);
                match (left, right) {
                    (Some(left), Some(right)) => {
                        let result = compatibility::binary_result(*operator, left, right);
                        if result.is_none() {
                            self.emit(
                                codes::OPERAND_TYPE,
                                format!("{operator:?} does not accept {left:?} and {right:?}."),
                                Some(expression.source()),
                                None,
                            );
                        }
                        result
                    }
                    _ => None,
                }
            }
        }?;
        if inferred != expression.result_type() {
            self.emit(codes::OPERAND_TYPE, format!("The declared expression type {:?} differs from its semantic type {inferred:?}.",
                expression.result_type()), Some(expression.source()), None);
            return None;
        }
        Some(inferred)
    }
    fn write_type(&mut self, id: SymbolId, source: SourceRef) -> Option<PlcType> {
        let t = self.symbol_type(id, source)?;
        if self.program.symbols().get(id).is_ok_and(|symbol| {
            symbol.scope() == Scope::Pou(self.program.entry())
                && symbol.category() == VariableCategory::Input
        }) {
            self.emit(
                codes::INVALID_WRITE_TARGET,
                "An Alpha entry-program input is read-only during execution.",
                Some(source),
                Some(RelatedIdentity::Symbol(id)),
            );
            return None;
        }
        Some(t)
    }
    fn statements(&mut self, statements: &[Statement], depth: usize) {
        for statement in statements {
            if !self.visit(Some(statement.source()), depth) {
                break;
            }
            match statement.kind() {
                StatementKind::Assign { target, value } => {
                    let target_type = self.write_type(*target, statement.source());
                    let value_type = self.expression(value, depth + 1);
                    if let (Some(target_type), Some(value_type)) = (target_type, value_type)
                        && compatibility::assignment(target_type, value_type)
                            != AssignmentCompatibility::Identity
                    {
                        self.emit(codes::ASSIGNMENT_TYPE, format!("Cannot assign {value_type:?} to {target_type:?}; Alpha assignments require identical types."),
                                Some(statement.source()), Some(RelatedIdentity::Symbol(*target)));
                    }
                }
                StatementKind::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    if let Some(t) = self.expression(condition, depth + 1)
                        && t != PlcType::Bool
                    {
                        self.emit(
                            codes::CONDITION_TYPE,
                            "A conditional requires a BOOL expression.",
                            Some(condition.source()),
                            None,
                        );
                    }
                    // Both paths must be valid even if the condition is a constant.
                    self.statements(then_body, depth + 1);
                    self.statements(else_body, depth + 1);
                }
                StatementKind::FunctionBlockCall(call) => {
                    self.call(call, statement.source(), depth);
                }
            }
        }
    }
}
