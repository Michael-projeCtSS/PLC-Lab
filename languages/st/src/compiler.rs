use crate::{
    ast,
    diagnostics::{CompilerConfig, Diagnostics, codes, error},
    literals,
};
use plc_ir::{CallBinding, FunctionBlockCall, Program, ValidatedProgram};
use plc_types::{
    PlcType,
    compatibility::{self, AssignmentCompatibility, UnaryOperator},
    diagnostics::Diagnostic,
    function_blocks::{
        FunctionBlockCatalog, FunctionBlockDefinition, FunctionBlockInstance, ParameterDirection,
    },
    identity::{
        DocumentId, FunctionBlockId, FunctionBlockInstanceId, PouId, SourceElementId, SymbolId,
    },
    source::{SourceLanguage, SourceLocation, SourceMap, SourceRef, TextSpan},
    symbols::{Pou, PouKind, Scope, Symbol, SymbolTable, VariableCategory},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationSummary {
    pub name: String,
    pub type_name: String,
    pub role: String,
    pub span: TextSpan,
}

/// An owned, immutable candidate that passed IR validation at construction.
/// Borrowed validation is explicit at the existing runtime-lowering boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledProgram {
    program: Program,
    name: String,
    declarations: Vec<DeclarationSummary>,
}
impl CompiledProgram {
    pub fn program_name(&self) -> &str {
        &self.name
    }
    pub fn declarations(&self) -> &[DeclarationSummary] {
        &self.declarations
    }
    pub fn program(&self) -> &Program {
        &self.program
    }
    pub fn validated(&self) -> Result<ValidatedProgram<'_>, Vec<Diagnostic>> {
        plc_ir::validate(&self.program)
    }
}

pub fn compile_st(
    source: &str,
    configuration: CompilerConfig,
) -> Result<CompiledProgram, Vec<Diagnostic>> {
    let ast = crate::parse_st(source, configuration)?;
    let owner = PouId::new(1).expect("fixed entry identity");
    let document = DocumentId::new(1).expect("fixed document identity");
    let mut compiler = Compiler {
        owner,
        document,
        symbols: SymbolTable::new(),
        catalog: FunctionBlockCatalog::new(),
        sources: SourceMap::new(),
        next_source: 1,
        next_symbol: 1,
        next_instance: 1,
        definition: None,
        instances: BTreeMap::new(),
        declarations: Vec::new(),
        errors: Diagnostics::new(configuration.diagnostics),
    };
    compiler
        .sources
        .register_document(document, SourceLanguage::St)
        .expect("fresh source registry");
    compiler.anchor(ast.span);
    let pou = Pou::new(owner, &ast.name.text, PouKind::Program).map_err(|e| {
        vec![
            e.diagnostic()
                .with_source(Some(crate::diagnostics::location(ast.name.span, 1))),
        ]
    })?;
    compiler
        .symbols
        .declare_pou(pou)
        .expect("fresh program namespace");
    compiler.declare(&ast.declarations);
    let body = compiler.body(&ast.body);
    if !compiler.errors.items.is_empty() {
        return Err(compiler.errors.items);
    }
    let program = Program::new(compiler.symbols, compiler.sources, owner, body)
        .with_function_blocks(compiler.catalog);
    if let Err(errors) = plc_ir::validate(&program) {
        for diagnostic in errors {
            compiler.errors.push(diagnostic);
        }
        return Err(compiler.errors.items);
    }
    Ok(CompiledProgram {
        program,
        name: ast.name.text,
        declarations: compiler.declarations,
    })
}

struct Compiler {
    owner: PouId,
    document: DocumentId,
    symbols: SymbolTable,
    catalog: FunctionBlockCatalog,
    sources: SourceMap,
    next_source: u32,
    next_symbol: u32,
    next_instance: u32,
    definition: Option<FunctionBlockId>,
    instances: BTreeMap<String, FunctionBlockInstanceId>,
    declarations: Vec<DeclarationSummary>,
    errors: Diagnostics,
}
impl Compiler {
    fn anchor(&mut self, span: TextSpan) -> SourceRef {
        let reference = SourceRef::new(
            self.document,
            SourceElementId::new(self.next_source).expect("bounded positive source counter"),
        );
        self.next_source += 1;
        self.sources
            .register_element(SourceLocation::new(
                reference,
                SourceLanguage::St,
                Some(span),
            ))
            .expect("fresh unique source anchor");
        reference
    }
    fn report(&mut self, code: &str, message: impl Into<String>, span: TextSpan) {
        if self.errors.stopped {
            return;
        }
        let reference = self.anchor(span);
        self.errors
            .push(error(code, message, None).with_source(self.sources.get(reference).cloned()));
    }
    fn declare(&mut self, declarations: &[ast::Declaration]) {
        let mut names = BTreeSet::new();
        for declaration in declarations {
            if self.errors.stopped {
                break;
            }
            let name = &declaration.name;
            if !names.insert(name.text.to_ascii_uppercase()) {
                self.report(codes::DUPLICATE, format!("Declaration '{}' conflicts with an existing variable or TON instance (names are case-insensitive).", name.text), name.span);
                continue;
            }
            let source = self.anchor(declaration.span);
            match declaration.kind {
                ast::DeclaredType::Scalar(kind) => {
                    let id = SymbolId::new(self.next_symbol).expect("bounded declaration count");
                    self.next_symbol += 1;
                    match Symbol::new(
                        id,
                        &name.text,
                        kind,
                        Scope::Pou(self.owner),
                        declaration.role,
                        Some(source),
                    )
                    .and_then(|s| self.symbols.declare(s))
                    {
                        Ok(()) => self.declarations.push(DeclarationSummary {
                            name: name.text.clone(),
                            type_name: type_name(kind).into(),
                            role: match declaration.role {
                                VariableCategory::Input => "Input",
                                VariableCategory::Output => "Output",
                                _ => "Local",
                            }
                            .into(),
                            span: declaration.span,
                        }),
                        Err(e) => self.errors.push(
                            e.diagnostic()
                                .with_source(self.sources.get(source).cloned()),
                        ),
                    }
                }
                ast::DeclaredType::Ton => {
                    if declaration.role != VariableCategory::Local {
                        self.report(
                            codes::BLOCK,
                            "TON instances must be declared in VAR, not VAR_INPUT or VAR_OUTPUT.",
                            declaration.span,
                        );
                        continue;
                    }
                    let definition = if let Some(id) = self.definition {
                        id
                    } else {
                        let id = FunctionBlockId::new(1).expect("first allocated definition");
                        self.catalog
                            .declare_definition(
                                FunctionBlockDefinition::ton(id).with_source(source),
                            )
                            .expect("one standard definition");
                        self.definition = Some(id);
                        id
                    };
                    let id = FunctionBlockInstanceId::new(self.next_instance)
                        .expect("bounded instance count");
                    self.next_instance += 1;
                    match FunctionBlockInstance::new(
                        id,
                        definition,
                        self.owner,
                        &name.text,
                        Some(source),
                    )
                    .and_then(|i| self.catalog.declare_instance(i))
                    {
                        Ok(()) => {
                            self.instances.insert(name.text.to_ascii_uppercase(), id);
                            self.declarations.push(DeclarationSummary {
                                name: name.text.clone(),
                                type_name: "TON".into(),
                                role: "FB Instance".into(),
                                span: declaration.span,
                            });
                        }
                        Err(e) => self.errors.push(
                            e.diagnostic()
                                .with_source(self.sources.get(source).cloned()),
                        ),
                    }
                }
            }
        }
    }
    fn resolve(&mut self, name: &ast::Name) -> Option<(SymbolId, PlcType, VariableCategory)> {
        match self.symbols.resolve(Scope::Pou(self.owner), &name.text) {
            Ok(symbol) => Some((symbol.id(), symbol.plc_type(), symbol.category())),
            Err(_) => {
                self.report(
                    codes::UNKNOWN,
                    format!("Unknown variable '{}'.", name.text),
                    name.span,
                );
                None
            }
        }
    }
    fn target(&mut self, name: &ast::Name, code: &str) -> Option<(SymbolId, PlcType)> {
        let (id, kind, role) = self.resolve(name)?;
        if role == VariableCategory::Input {
            self.report(
                code,
                format!(
                    "Cannot write VAR_INPUT '{}'; inputs are read-only.",
                    name.text
                ),
                name.span,
            );
            None
        } else {
            Some((id, kind))
        }
    }
    fn body(&mut self, statements: &[ast::Statement]) -> Vec<plc_ir::Statement> {
        let mut body = Vec::new();
        for statement in statements {
            if self.errors.stopped {
                break;
            }
            if let Some(statement) = self.statement(statement) {
                body.push(statement);
            }
        }
        body
    }
    fn statement(&mut self, statement: &ast::Statement) -> Option<plc_ir::Statement> {
        let source = self.anchor(statement.span);
        match &statement.kind {
            ast::StatementKind::Assignment(name, expression) => {
                let target = self.target(name, codes::TARGET);
                let value = self.expression(expression);
                let ((target, target_type), value) = (target?, value?);
                if compatibility::assignment(target_type, value.result_type())
                    != AssignmentCompatibility::Identity
                {
                    self.report(
                        codes::TYPE,
                        format!(
                            "Cannot assign {} to {} '{}'; Alpha has no implicit conversions.",
                            type_name(value.result_type()),
                            type_name(target_type),
                            name.text
                        ),
                        expression.span,
                    );
                    return None;
                }
                Some(plc_ir::Statement::assign(source, target, value))
            }
            ast::StatementKind::If(condition, then_body, else_body) => {
                let value = self.expression(condition);
                let then_body = self.body(then_body);
                let else_body = self.body(else_body);
                let value = value?;
                if value.result_type() != PlcType::Bool {
                    self.report(codes::TYPE, "IF condition must be BOOL.", condition.span);
                    return None;
                }
                Some(plc_ir::Statement::conditional(
                    source, value, then_body, else_body,
                ))
            }
            ast::StatementKind::Call(name, associations) => self
                .call(name, associations)
                .map(|call| plc_ir::Statement::function_block_call(source, call)),
        }
    }
    fn expression(&mut self, expression: &ast::Expression) -> Option<plc_ir::Expression> {
        if self.errors.stopped {
            return None;
        }
        let source = self.anchor(expression.span);
        match &expression.kind {
            ast::ExpressionKind::Literal(literal) => {
                self.literal(source, literal, false, expression.span)
            }
            ast::ExpressionKind::Identifier(name) => {
                let (id, kind, _) = self.resolve(name)?;
                Some(plc_ir::Expression::read(source, id, kind))
            }
            ast::ExpressionKind::Parentheses(operand) => self.expression(operand),
            ast::ExpressionKind::Unary(operator, operand) => {
                if *operator == UnaryOperator::Negate
                    && let Some(literal) = signed_literal(operand)
                {
                    return self.literal(source, literal, true, expression.span);
                }
                let value = self.expression(operand)?;
                let Some(kind) = compatibility::unary_result(*operator, value.result_type()) else {
                    self.report(
                        codes::TYPE,
                        format!(
                            "Unary {operator:?} cannot accept {}.",
                            type_name(value.result_type())
                        ),
                        expression.span,
                    );
                    return None;
                };
                Some(plc_ir::Expression::unary(source, kind, *operator, value))
            }
            ast::ExpressionKind::Binary(operator, left, right) => {
                let left_value = self.expression(left);
                let right_value = self.expression(right);
                let (left_value, right_value) = (left_value?, right_value?);
                let Some(kind) = compatibility::binary_result(
                    *operator,
                    left_value.result_type(),
                    right_value.result_type(),
                ) else {
                    self.report(codes::TYPE, format!("{operator:?} does not accept {} and {}; operand types must follow the Alpha type rules.", type_name(left_value.result_type()), type_name(right_value.result_type())), expression.span);
                    return None;
                };
                Some(plc_ir::Expression::binary(
                    source,
                    kind,
                    *operator,
                    left_value,
                    right_value,
                ))
            }
        }
    }
    fn literal(
        &mut self,
        source: SourceRef,
        literal: &ast::Literal,
        negative: bool,
        span: TextSpan,
    ) -> Option<plc_ir::Expression> {
        match literals::value(literal, negative) {
            Ok(value) => Some(plc_ir::Expression::constant(source, value)),
            Err(message) => {
                self.report(codes::LITERAL, message, span);
                None
            }
        }
    }
    fn call(
        &mut self,
        name: &ast::Name,
        associations: &[ast::Association],
    ) -> Option<FunctionBlockCall> {
        let Some(instance) = self.instances.get(&name.text.to_ascii_uppercase()).copied() else {
            self.report(
                codes::BLOCK,
                format!(
                    "Unknown TON instance '{}'; declare it in VAR first.",
                    name.text
                ),
                name.span,
            );
            return None;
        };
        let parameters = self
            .catalog
            .definition(
                self.catalog
                    .instance(instance)
                    .expect("declared instance")
                    .definition(),
            )
            .expect("declared definition")
            .parameters()
            .to_vec();
        let before = self.errors.items.len();
        let mut seen = BTreeSet::new();
        let mut bindings = Vec::new();
        for association in associations {
            if self.errors.stopped {
                break;
            }
            let Some(parameter) = parameters
                .iter()
                .find(|p| p.name().eq_ignore_ascii_case(&association.parameter.text))
            else {
                self.report(
                    codes::BLOCK,
                    format!(
                        "Unknown TON parameter '{}'; use IN, PT, Q or ET.",
                        association.parameter.text
                    ),
                    association.parameter.span,
                );
                continue;
            };
            if !seen.insert(parameter.id()) {
                self.report(
                    codes::BLOCK,
                    format!(
                        "TON parameter '{}' is associated more than once.",
                        association.parameter.text
                    ),
                    association.parameter.span,
                );
                continue;
            }
            match (&association.value, parameter.direction()) {
                (ast::AssociationValue::Input(expression), ParameterDirection::Input) => {
                    if let Some(value) = self.expression(expression) {
                        if value.result_type() != parameter.plc_type() {
                            self.report(
                                codes::BLOCK,
                                format!(
                                    "TON {} requires {}, received {}.",
                                    parameter.name(),
                                    type_name(parameter.plc_type()),
                                    type_name(value.result_type())
                                ),
                                expression.span,
                            );
                        } else {
                            bindings.push(CallBinding::input(parameter.id(), value));
                        }
                    }
                }
                (ast::AssociationValue::Output(target), ParameterDirection::Output) => {
                    if let Some((id, kind)) = self.target(target, codes::BLOCK) {
                        if kind != parameter.plc_type() {
                            self.report(
                                codes::BLOCK,
                                format!(
                                    "TON {} target must be {}, received {}.",
                                    parameter.name(),
                                    type_name(parameter.plc_type()),
                                    type_name(kind)
                                ),
                                target.span,
                            );
                        } else {
                            bindings.push(CallBinding::output(parameter.id(), id));
                        }
                    }
                }
                _ => self.report(
                    codes::BLOCK,
                    format!(
                        "Use {} for TON {}.",
                        if parameter.direction() == ParameterDirection::Input {
                            ":="
                        } else {
                            "=>"
                        },
                        parameter.name()
                    ),
                    association.span,
                ),
            }
        }
        for parameter in parameters.iter().filter(|p| p.required()) {
            if !seen.contains(&parameter.id()) {
                self.report(
                    codes::BLOCK,
                    format!("Required TON input {} is missing.", parameter.name()),
                    name.span,
                );
            }
        }
        if self.errors.items.len() != before {
            return None;
        }
        match FunctionBlockCall::new(instance, bindings) {
            Ok(call) => Some(call),
            Err(_) => {
                self.report(
                    codes::BLOCK,
                    "TON parameters must be associated at most once.",
                    name.span,
                );
                None
            }
        }
    }
}
fn signed_literal(expression: &ast::Expression) -> Option<&ast::Literal> {
    match &expression.kind {
        ast::ExpressionKind::Literal(
            literal @ (ast::Literal::Integer(_) | ast::Literal::Real(_)),
        ) => Some(literal),
        ast::ExpressionKind::Parentheses(inner) => signed_literal(inner),
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
