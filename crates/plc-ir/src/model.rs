use plc_types::{
    PlcType, PlcValue,
    compatibility::{BinaryOperator, UnaryOperator},
    diagnostics::{Diagnostic, DiagnosticCategory, DiagnosticCode, Severity, codes},
    function_blocks::{FunctionBlockCatalog, ParameterDirection},
    identity::{FunctionBlockInstanceId, ParameterId, PouId, SymbolId},
    source::{SourceMap, SourceRef},
    symbols::SymbolTable,
};
use std::collections::BTreeSet;

/// A claimed result type is checked by validation, never trusted by future execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    source: SourceRef,
    result_type: PlcType,
    kind: ExpressionKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    Constant(PlcValue),
    Read(SymbolId),
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}
impl Expression {
    pub fn constant(source: SourceRef, value: PlcValue) -> Self {
        Self {
            source,
            result_type: value.plc_type(),
            kind: ExpressionKind::Constant(value),
        }
    }
    pub fn read(source: SourceRef, symbol: SymbolId, result_type: PlcType) -> Self {
        Self {
            source,
            result_type,
            kind: ExpressionKind::Read(symbol),
        }
    }
    pub fn unary(
        source: SourceRef,
        result_type: PlcType,
        operator: UnaryOperator,
        operand: Self,
    ) -> Self {
        Self {
            source,
            result_type,
            kind: ExpressionKind::Unary {
                operator,
                operand: Box::new(operand),
            },
        }
    }
    pub fn binary(
        source: SourceRef,
        result_type: PlcType,
        operator: BinaryOperator,
        left: Self,
        right: Self,
    ) -> Self {
        Self {
            source,
            result_type,
            kind: ExpressionKind::Binary {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            },
        }
    }
    pub fn source(&self) -> SourceRef {
        self.source
    }
    pub fn result_type(&self) -> PlcType {
        self.result_type
    }
    pub fn kind(&self) -> &ExpressionKind {
        &self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    source: SourceRef,
    kind: StatementKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    Assign {
        target: SymbolId,
        value: Expression,
    },
    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    FunctionBlockCall(FunctionBlockCall),
}
impl Statement {
    pub fn assign(source: SourceRef, target: SymbolId, value: Expression) -> Self {
        Self {
            source,
            kind: StatementKind::Assign { target, value },
        }
    }
    pub fn conditional(
        source: SourceRef,
        condition: Expression,
        then_body: Vec<Self>,
        else_body: Vec<Self>,
    ) -> Self {
        Self {
            source,
            kind: StatementKind::If {
                condition,
                then_body,
                else_body,
            },
        }
    }
    pub fn function_block_call(source: SourceRef, call: FunctionBlockCall) -> Self {
        Self {
            source,
            kind: StatementKind::FunctionBlockCall(call),
        }
    }
    pub fn source(&self) -> SourceRef {
        self.source
    }
    pub fn kind(&self) -> &StatementKind {
        &self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallBinding {
    Input {
        parameter: ParameterId,
        value: Expression,
    },
    Output {
        parameter: ParameterId,
        target: SymbolId,
    },
    /// Reserved semantic association; TON signatures never accept InOut.
    InOut {
        parameter: ParameterId,
        target: SymbolId,
    },
}
impl CallBinding {
    pub fn input(parameter: ParameterId, value: Expression) -> Self {
        Self::Input { parameter, value }
    }
    pub fn output(parameter: ParameterId, target: SymbolId) -> Self {
        Self::Output { parameter, target }
    }
    pub fn in_out(parameter: ParameterId, target: SymbolId) -> Self {
        Self::InOut { parameter, target }
    }
    pub fn parameter(&self) -> ParameterId {
        match self {
            Self::Input { parameter, .. }
            | Self::Output { parameter, .. }
            | Self::InOut { parameter, .. } => *parameter,
        }
    }
    pub fn direction(&self) -> ParameterDirection {
        match self {
            Self::Input { .. } => ParameterDirection::Input,
            Self::Output { .. } => ParameterDirection::Output,
            Self::InOut { .. } => ParameterDirection::InOut,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateCallParameter;
impl DuplicateCallParameter {
    pub fn diagnostic(self) -> Diagnostic {
        Diagnostic::new(
            DiagnosticCode::new(codes::DUPLICATE_FB_BINDING).expect("fixed code"),
            DiagnosticCategory::CompilerDiagnostic,
            Severity::Error,
            "A function-block parameter cannot be bound more than once.",
        )
        .expect("fixed explanation")
    }
}
impl std::fmt::Display for DuplicateCallParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("A call parameter cannot be bound more than once.")
    }
}
impl std::error::Error for DuplicateCallParameter {}

/// The instance declaration is the only authority for the called definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBlockCall {
    instance: FunctionBlockInstanceId,
    bindings: Vec<CallBinding>,
}
impl FunctionBlockCall {
    pub fn new(
        instance: FunctionBlockInstanceId,
        bindings: Vec<CallBinding>,
    ) -> Result<Self, DuplicateCallParameter> {
        let mut parameters = BTreeSet::new();
        for id in bindings.iter().map(CallBinding::parameter) {
            if !parameters.insert(id) {
                return Err(DuplicateCallParameter);
            }
        }
        Ok(Self { instance, bindings })
    }
    pub fn instance(&self) -> FunctionBlockInstanceId {
        self.instance
    }
    pub fn bindings(&self) -> &[CallBinding] {
        &self.bindings
    }
}

/// An immutable candidate. Cross-reference/type errors must pass through `validate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    symbols: SymbolTable,
    sources: SourceMap,
    entry: PouId,
    body: Vec<Statement>,
    function_blocks: FunctionBlockCatalog,
}
impl Program {
    pub fn new(
        symbols: SymbolTable,
        sources: SourceMap,
        entry: PouId,
        body: Vec<Statement>,
    ) -> Self {
        Self {
            symbols,
            sources,
            entry,
            body,
            function_blocks: FunctionBlockCatalog::new(),
        }
    }
    pub fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }
    pub fn with_function_blocks(mut self, catalog: FunctionBlockCatalog) -> Self {
        self.function_blocks = catalog;
        self
    }
    pub fn function_blocks(&self) -> &FunctionBlockCatalog {
        &self.function_blocks
    }
    pub fn sources(&self) -> &SourceMap {
        &self.sources
    }
    pub fn entry(&self) -> PouId {
        self.entry
    }
    pub fn body(&self) -> &[Statement] {
        &self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plc_types::{
        function_blocks::{FunctionBlockDefinition, FunctionBlockInstance, TonParameter},
        identity::{DocumentId, SourceElementId},
        source::{SourceLanguage, SourceLocation},
        symbols::{Pou, PouKind},
    };
    #[test]
    fn validator_defensively_rejects_duplicate_binding_with_call_source() {
        let owner = PouId::new(1).unwrap();
        let instance = FunctionBlockInstanceId::new(1).unwrap();
        let definition = plc_types::identity::FunctionBlockId::new(77).unwrap();
        let source = SourceRef::new(
            DocumentId::new(1).unwrap(),
            SourceElementId::new(1).unwrap(),
        );
        let mut sources = SourceMap::new();
        sources
            .register_document(source.document_id(), SourceLanguage::St)
            .unwrap();
        sources
            .register_element(SourceLocation::new(source, SourceLanguage::St, None))
            .unwrap();
        let mut symbols = SymbolTable::new();
        symbols
            .declare_pou(Pou::new(owner, "Main", PouKind::Program).unwrap())
            .unwrap();
        let mut catalog = FunctionBlockCatalog::new();
        catalog
            .declare_definition(FunctionBlockDefinition::ton(definition))
            .unwrap();
        catalog
            .declare_instance(
                FunctionBlockInstance::new(instance, definition, owner, "Timer", None).unwrap(),
            )
            .unwrap();
        // Private test corruption proves validation has its own guard. Public construction rejects this.
        let binding = CallBinding::input(
            TonParameter::In.id(),
            Expression::constant(source, PlcValue::Bool(false)),
        );
        let mut call = FunctionBlockCall::new(
            instance,
            vec![
                binding.clone(),
                CallBinding::input(
                    TonParameter::Pt.id(),
                    Expression::constant(source, PlcValue::Time(plc_types::PlcTime::ZERO)),
                ),
            ],
        )
        .unwrap();
        call.bindings.push(binding);
        let program = Program::new(
            symbols,
            sources,
            owner,
            vec![Statement::function_block_call(source, call)],
        )
        .with_function_blocks(catalog);
        let errors = crate::validate(&program).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), codes::DUPLICATE_FB_BINDING);
        assert_eq!(errors[0].source().unwrap().reference(), source);
    }
}
