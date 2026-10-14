//! Checked ownership transition; never stores a borrowed Program or ValidatedProgram.

use crate::errors::{InternalError, LoweringError, diagnostic};
use plc_ir::{
    CallBinding, Expression, ExpressionKind, FunctionBlockCall, Program, Statement, StatementKind,
    ValidatedProgram,
};
use plc_types::{
    PlcType, PlcValue,
    compatibility::{BinaryOperator, UnaryOperator},
    diagnostics::{DiagnosticCategory, RelatedIdentity, codes},
    function_blocks::{FunctionBlockKind, StandardFunctionBlock, TonParameter},
    identity::{FunctionBlockInstanceId, SymbolId},
    source::{SourceMap, SourceRef},
    symbols::{Scope, VariableCategory},
};
use std::collections::BTreeMap;

/// Explicit compiled entry roles, independent of universal POU category semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRole {
    ExternalInput,
    ExternalOutput,
    Internal,
}
#[derive(Debug)]
pub(crate) struct Slot {
    pub(crate) role: MemoryRole,
    pub(crate) plc_type: PlcType,
}

#[derive(Debug)]
pub struct ExecutionPlan {
    pub(crate) layout: BTreeMap<SymbolId, Slot>,
    pub(crate) sources: SourceMap,
    pub(crate) body: Vec<ExecStatement>,
    pub(crate) instances: BTreeMap<FunctionBlockInstanceId, StandardFunctionBlock>,
}
impl ExecutionPlan {
    pub fn memory_role(&self, id: SymbolId) -> Option<MemoryRole> {
        self.layout.get(&id).map(|s| s.role)
    }
}
#[derive(Debug)]
pub(crate) struct ExecExpression {
    pub(crate) source: SourceRef,
    pub(crate) result_type: PlcType,
    pub(crate) kind: ExecExpressionKind,
}
#[derive(Debug)]
pub(crate) enum ExecExpressionKind {
    Constant(PlcValue),
    Read(SymbolId),
    Unary(UnaryOperator, Box<ExecExpression>),
    Binary(BinaryOperator, Box<ExecExpression>, Box<ExecExpression>),
}
#[derive(Debug)]
pub(crate) struct ExecStatement {
    pub(crate) source: SourceRef,
    pub(crate) kind: ExecStatementKind,
}
#[derive(Debug)]
pub(crate) enum ExecStatementKind {
    Assign(SymbolId, ExecExpression),
    If(ExecExpression, Vec<ExecStatement>, Vec<ExecStatement>),
    TonCall {
        instance: FunctionBlockInstanceId,
        input: ExecExpression,
        preset: ExecExpression,
        q_target: Option<SymbolId>,
        et_target: Option<SymbolId>,
    },
}

pub fn lower(validated: &ValidatedProgram<'_>) -> Result<ExecutionPlan, LoweringError> {
    let program = validated.program();
    let mut layout = BTreeMap::new();
    for symbol in program.symbols().symbols() {
        let role = match (symbol.scope(), symbol.category()) {
            (Scope::Global, VariableCategory::Local) => MemoryRole::Internal,
            (Scope::Global, _) => {
                return Err(LoweringError::Internal(InternalError::new(
                    "A validated global declaration has an invalid category.",
                )));
            }
            (Scope::Pou(owner), _) if owner != program.entry() => continue,
            (_, VariableCategory::Input) => MemoryRole::ExternalInput,
            (_, VariableCategory::Output) => MemoryRole::ExternalOutput,
            (_, VariableCategory::Local) => MemoryRole::Internal,
            (_, VariableCategory::InOut) => {
                return Err(LoweringError::Unsupported(
                    diagnostic(
                        codes::UNSUPPORTED_DECLARATION,
                        DiagnosticCategory::CompilerDiagnostic,
                        "Alpha entry-program InOut declarations are not executable.",
                    )
                    .with_source(
                        symbol
                            .source()
                            .and_then(|r| program.sources().get(r))
                            .cloned(),
                    )
                    .with_related(RelatedIdentity::Symbol(symbol.id())),
                ));
            }
        };
        layout.insert(
            symbol.id(),
            Slot {
                role,
                plc_type: symbol.plc_type(),
            },
        );
    }
    let mut sources = SourceMap::new();
    let body = lower_body(program.body(), program, &mut sources)?;
    let mut instances = BTreeMap::new();
    for instance in program
        .function_blocks()
        .instances()
        .filter(|i| i.owner() == program.entry())
    {
        let definition = program
            .function_blocks()
            .definition(instance.definition())
            .ok_or_else(|| {
                LoweringError::Internal(InternalError::new(
                    "A validated instance has no definition.",
                ))
            })?;
        if let FunctionBlockKind::Standard(kind) = definition.kind() {
            instances.insert(instance.id(), kind);
        }
    }
    Ok(ExecutionPlan {
        layout,
        sources,
        body,
        instances,
    })
}

fn retain_source(
    reference: SourceRef,
    original: &SourceMap,
    retained: &mut SourceMap,
) -> Result<(), LoweringError> {
    if retained.get(reference).is_some() {
        return Ok(());
    }
    let location = original.get(reference).ok_or_else(|| {
        LoweringError::Internal(InternalError::new(
            "A validated execution source is not registered.",
        ))
    })?;
    if retained
        .document_language(reference.document_id())
        .is_none()
    {
        retained
            .register_document(reference.document_id(), location.language())
            .map_err(|_| {
                LoweringError::Internal(InternalError::new(
                    "Execution source document invariants are inconsistent.",
                ))
            })?;
    }
    retained.register_element(location.clone()).map_err(|_| {
        LoweringError::Internal(InternalError::new(
            "Execution source element invariants are inconsistent.",
        ))
    })
}

fn lower_expression(
    expression: &Expression,
    original: &SourceMap,
    retained: &mut SourceMap,
) -> Result<ExecExpression, LoweringError> {
    retain_source(expression.source(), original, retained)?;
    let kind = match expression.kind() {
        ExpressionKind::Constant(v) => ExecExpressionKind::Constant(*v),
        ExpressionKind::Read(id) => ExecExpressionKind::Read(*id),
        ExpressionKind::Unary { operator, operand } => ExecExpressionKind::Unary(
            *operator,
            Box::new(lower_expression(operand, original, retained)?),
        ),
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => ExecExpressionKind::Binary(
            *operator,
            Box::new(lower_expression(left, original, retained)?),
            Box::new(lower_expression(right, original, retained)?),
        ),
    };
    Ok(ExecExpression {
        source: expression.source(),
        result_type: expression.result_type(),
        kind,
    })
}
fn lower_body(
    body: &[Statement],
    program: &Program,
    retained: &mut SourceMap,
) -> Result<Vec<ExecStatement>, LoweringError> {
    body.iter()
        .map(|s| {
            let sources = program.sources();
            retain_source(s.source(), sources, retained)?;
            let kind = match s.kind() {
                StatementKind::Assign { target, value } => {
                    ExecStatementKind::Assign(*target, lower_expression(value, sources, retained)?)
                }
                StatementKind::If {
                    condition,
                    then_body,
                    else_body,
                } => ExecStatementKind::If(
                    lower_expression(condition, sources, retained)?,
                    lower_body(then_body, program, retained)?,
                    lower_body(else_body, program, retained)?,
                ),
                StatementKind::FunctionBlockCall(call) => {
                    lower_call(call, s.source(), program, retained)?
                }
            };
            Ok(ExecStatement {
                source: s.source(),
                kind,
            })
        })
        .collect()
}

fn lower_call(
    call: &FunctionBlockCall,
    source: SourceRef,
    program: &Program,
    retained: &mut SourceMap,
) -> Result<ExecStatementKind, LoweringError> {
    let instance = program
        .function_blocks()
        .instance(call.instance())
        .ok_or_else(|| {
            LoweringError::Internal(InternalError::new("A validated call has no instance."))
        })?;
    let definition = program
        .function_blocks()
        .definition(instance.definition())
        .ok_or_else(|| {
            LoweringError::Internal(InternalError::new("A validated call has no definition."))
        })?;
    if definition.kind() != FunctionBlockKind::Standard(StandardFunctionBlock::Ton) {
        return Err(LoweringError::Unsupported(
            diagnostic(
                codes::UNSUPPORTED_FB_KIND,
                DiagnosticCategory::CompilerDiagnostic,
                "This function-block kind is not executable.",
            )
            .with_source(program.sources().get(source).cloned()),
        ));
    }
    let input = |parameter: TonParameter| -> Result<&Expression, LoweringError> {
        match call
            .bindings()
            .iter()
            .find(|b| b.parameter() == parameter.id())
        {
            Some(CallBinding::Input { value, .. }) => Ok(value),
            _ => Err(LoweringError::Internal(InternalError::new(
                "A validated TON input binding is missing or inconsistent.",
            ))),
        }
    };
    let output = |parameter: TonParameter| -> Result<Option<SymbolId>, LoweringError> {
        match call
            .bindings()
            .iter()
            .find(|b| b.parameter() == parameter.id())
        {
            Some(CallBinding::Output { target, .. }) => Ok(Some(*target)),
            None => Ok(None),
            _ => Err(LoweringError::Internal(InternalError::new(
                "A validated TON output binding is inconsistent.",
            ))),
        }
    };
    // Canonical signature order: producer binding order never reaches the executor.
    Ok(ExecStatementKind::TonCall {
        instance: call.instance(),
        input: lower_expression(input(TonParameter::In)?, program.sources(), retained)?,
        preset: lower_expression(input(TonParameter::Pt)?, program.sources(), retained)?,
        q_target: output(TonParameter::Q)?,
        et_target: output(TonParameter::Et)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use plc_ir::FunctionBlockCall;
    use plc_types::{
        function_blocks::{FunctionBlockCatalog, FunctionBlockDefinition, FunctionBlockInstance},
        identity::{DocumentId, FunctionBlockId, FunctionBlockInstanceId, PouId, SourceElementId},
        source::{SourceLanguage, SourceLocation},
        symbols::{Pou, PouKind, SymbolTable},
    };
    fn reference() -> SourceRef {
        SourceRef::new(
            DocumentId::new(1).unwrap(),
            SourceElementId::new(1).unwrap(),
        )
    }
    #[test]
    fn user_definition_is_rejected_by_lowering_even_with_numeric_id_one() {
        let source = reference();
        let mut original = SourceMap::new();
        original
            .register_document(source.document_id(), SourceLanguage::St)
            .unwrap();
        original
            .register_element(SourceLocation::new(source, SourceLanguage::St, None))
            .unwrap();
        let definition = FunctionBlockId::new(1).unwrap();
        let instance = FunctionBlockInstanceId::new(1).unwrap();
        let owner = PouId::new(1).unwrap();
        let mut catalog = FunctionBlockCatalog::new();
        catalog
            .declare_definition(FunctionBlockDefinition::user_defined(definition, vec![]).unwrap())
            .unwrap();
        catalog
            .declare_instance(
                FunctionBlockInstance::new(instance, definition, owner, "Custom", None).unwrap(),
            )
            .unwrap();
        let mut symbols = SymbolTable::new();
        symbols
            .declare_pou(Pou::new(owner, "Main", PouKind::Program).unwrap())
            .unwrap();
        let call = FunctionBlockCall::new(instance, vec![]).unwrap();
        let body = vec![Statement::function_block_call(source, call)];
        let program = Program::new(symbols, original, owner, body).with_function_blocks(catalog);
        let error = lower_body(program.body(), &program, &mut SourceMap::new()).unwrap_err();
        assert!(matches!(error, LoweringError::Unsupported(_)));
        assert_eq!(error.diagnostic().code(), "PLC-COMP-023");
        assert_eq!(
            error.diagnostic().category(),
            DiagnosticCategory::CompilerDiagnostic
        );
        assert!(error.diagnostic().source().is_some());
    }
    #[test]
    fn missing_validated_source_is_an_internal_lowering_invariant_error() {
        let error =
            retain_source(reference(), &SourceMap::new(), &mut SourceMap::new()).unwrap_err();
        assert!(matches!(error, LoweringError::Internal(_)));
        assert_eq!(error.diagnostic().code(), "APP-CORE-001");
        assert_eq!(
            error.diagnostic().category(),
            DiagnosticCategory::InternalApplicationError
        );
    }
}
