use crate::{
    ast,
    diagnostics::{CompilerConfig, Diagnostics, codes, error},
    literals,
};
use plc_ir::{CallBinding, FunctionBlockCall, Program, ValidatedProgram};
use plc_types::{
    PlcType, PlcValue,
    compatibility::{self, AssignmentCompatibility, BinaryOperator, ConversionFunction, UnaryOperator},
    diagnostics::Diagnostic,
    function_blocks::{
        FunctionBlockCatalog, FunctionBlockDefinition, FunctionBlockInstance, ParameterDirection, StandardFunctionBlock,
    },
    identity::{DocumentId, FunctionBlockId, FunctionBlockInstanceId, ParameterId, PouId, SourceElementId, SymbolId},
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

pub fn compile_st(source: &str, configuration: CompilerConfig) -> Result<CompiledProgram, Vec<Diagnostic>> {
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
        definitions: BTreeMap::new(),
        instances: BTreeMap::new(),
        declarations: Vec::new(),
        errors: Diagnostics::new(configuration.diagnostics),
    };
    compiler
        .sources
        .register_document(document, SourceLanguage::St)
        .expect("fresh source registry");
    compiler.anchor(ast.span);
    let pou = Pou::new(owner, &ast.name.text, PouKind::Program)
        .map_err(|e| vec![e.diagnostic().with_source(Some(crate::diagnostics::location(ast.name.span, 1)))])?;
    compiler.symbols.declare_pou(pou).expect("fresh program namespace");
    compiler.declare(&ast.declarations);
    let body = compiler.body(&ast.body);
    if !compiler.errors.items.is_empty() {
        return Err(compiler.errors.items);
    }
    let program = Program::new(compiler.symbols, compiler.sources, owner, body).with_function_blocks(compiler.catalog);
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
    definitions: BTreeMap<StandardFunctionBlock, FunctionBlockId>,
    instances: BTreeMap<String, (FunctionBlockInstanceId, StandardFunctionBlock)>,
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
            .register_element(SourceLocation::new(reference, SourceLanguage::St, Some(span)))
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
                self.report(
                    codes::DUPLICATE,
                    format!(
                        "Declaration '{}' conflicts with an existing variable or block instance (names are case-insensitive).",
                        name.text
                    ),
                    name.span,
                );
                continue;
            }
            let source = self.anchor(declaration.span);
            match declaration.kind {
                ast::DeclaredType::Scalar(kind) => {
                    let id = SymbolId::new(self.next_symbol).expect("bounded declaration count");
                    self.next_symbol += 1;
                    match Symbol::new(id, &name.text, kind, Scope::Pou(self.owner), declaration.role, Some(source))
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
                        Err(e) => self.errors.push(e.diagnostic().with_source(self.sources.get(source).cloned())),
                    }
                }
                ast::DeclaredType::FunctionBlock(kind) => {
                    if declaration.role != VariableCategory::Local {
                        self.report(
                            codes::BLOCK,
                            format!("{} instances must be declared in VAR, not VAR_INPUT or VAR_OUTPUT.", kind.name()),
                            declaration.span,
                        );
                        continue;
                    }
                    let definition = if let Some(id) = self.definitions.get(&kind) {
                        *id
                    } else {
                        let id = FunctionBlockId::new(u32::try_from(self.definitions.len() + 1).unwrap_or(u32::MAX))
                            .expect("nonzero definition identity");
                        self.catalog
                            .declare_definition(FunctionBlockDefinition::standard(id, kind).with_source(source))
                            .expect("one definition per standard kind");
                        self.definitions.insert(kind, id);
                        id
                    };
                    let id = FunctionBlockInstanceId::new(self.next_instance).expect("bounded instance count");
                    self.next_instance += 1;
                    match FunctionBlockInstance::new(id, definition, self.owner, &name.text, Some(source))
                        .and_then(|i| self.catalog.declare_instance(i))
                    {
                        Ok(()) => {
                            self.instances.insert(name.text.to_ascii_uppercase(), (id, kind));
                            self.declarations.push(DeclarationSummary {
                                name: name.text.clone(),
                                type_name: kind.name().into(),
                                role: "FB Instance".into(),
                                span: declaration.span,
                            });
                        }
                        Err(e) => self.errors.push(e.diagnostic().with_source(self.sources.get(source).cloned())),
                    }
                }
            }
        }
    }
    fn resolve(&mut self, name: &ast::Name) -> Option<(SymbolId, PlcType, VariableCategory)> {
        match self.symbols.resolve(Scope::Pou(self.owner), &name.text) {
            Ok(symbol) => Some((symbol.id(), symbol.plc_type(), symbol.category())),
            Err(_) => {
                let message = if self.instances.contains_key(&name.text.to_ascii_uppercase()) {
                    format!("'{}' is a function-block instance; read one of its outputs, for example {}.Q.", name.text, name.text)
                } else {
                    format!("Unknown variable '{}'.", name.text)
                };
                self.report(codes::UNKNOWN, message, name.span);
                None
            }
        }
    }
    fn target(&mut self, name: &ast::Name, code: &str) -> Option<(SymbolId, PlcType)> {
        let (id, kind, role) = self.resolve(name)?;
        if role == VariableCategory::Input {
            self.report(code, format!("Cannot write VAR_INPUT '{}'; inputs are read-only.", name.text), name.span);
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
    fn condition(&mut self, expression: &ast::Expression, what: &str) -> Option<plc_ir::Expression> {
        let value = self.expression(expression)?;
        if value.result_type() != PlcType::Bool {
            self.report(codes::TYPE, format!("{what} must be BOOL."), expression.span);
            return None;
        }
        Some(value)
    }
    fn statement(&mut self, statement: &ast::Statement) -> Option<plc_ir::Statement> {
        let source = self.anchor(statement.span);
        match &statement.kind {
            ast::StatementKind::Assignment(name, expression) => {
                let target = self.target(name, codes::TARGET);
                let value = self.expression(expression);
                let ((target, target_type), value) = (target?, value?);
                if compatibility::assignment(target_type, value.result_type()) != AssignmentCompatibility::Identity {
                    self.report(
                        codes::TYPE,
                        format!(
                            "Cannot assign {} to {} '{}'; Alpha has no implicit conversions. Use INT_TO_REAL, REAL_TO_INT or TRUNC.",
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
            ast::StatementKind::If(branches, else_body) => {
                // Lower IF/ELSIF/ELSE into nested conditionals, innermost last.
                let mut compiled = Vec::with_capacity(branches.len());
                let mut ok = true;
                for branch in branches {
                    let condition = self.condition(&branch.condition, "IF and ELSIF conditions");
                    let body = self.body(&branch.body);
                    match condition {
                        Some(c) => compiled.push((branch.span, c, body)),
                        None => ok = false,
                    }
                }
                let mut tail = self.body(else_body);
                if !ok {
                    return None;
                }
                let (_, first_condition, first_body) = compiled.remove(0);
                for (span, condition, body) in compiled.into_iter().rev() {
                    let branch_source = self.anchor(span);
                    tail = vec![plc_ir::Statement::conditional(branch_source, condition, body, tail)];
                }
                Some(plc_ir::Statement::conditional(source, first_condition, first_body, tail))
            }
            ast::StatementKind::Case(selector, arms, else_body) => self.case(source, selector, arms, else_body),
            ast::StatementKind::Call(name, associations) => self
                .call(name, associations)
                .map(|call| plc_ir::Statement::function_block_call(source, call)),
        }
    }
    fn case(
        &mut self,
        source: SourceRef,
        selector: &ast::Expression,
        arms: &[ast::CaseArm],
        else_body: &[ast::Statement],
    ) -> Option<plc_ir::Statement> {
        let probe = self.expression(selector)?;
        if probe.result_type() != PlcType::Int {
            self.report(codes::TYPE, "CASE selector must be INT in Alpha Structured Text.", selector.span);
            return None;
        }
        let mut seen: Vec<(i64, i64)> = Vec::new();
        let mut branches = Vec::new();
        for arm in arms {
            let mut tests = Vec::new();
            for label in &arm.labels {
                let (lo, hi, span) = match label {
                    ast::CaseLabel::Value(v, span) => (*v, *v, *span),
                    ast::CaseLabel::Range(lo, hi, span) => (*lo, *hi, *span),
                };
                if lo > hi || lo < i64::from(i16::MIN) || hi > i64::from(i16::MAX) {
                    self.report(codes::LITERAL, "CASE label range must be ascending INT values.", span);
                    return None;
                }
                if seen.iter().any(|(a, b)| lo <= *b && *a <= hi) {
                    self.report(codes::DUPLICATE, "CASE labels must not overlap.", span);
                    return None;
                }
                seen.push((lo, hi));
                let label_source = self.anchor(span);
                let compare = |me: &mut Self, op: BinaryOperator, value: i64| -> Option<plc_ir::Expression> {
                    let left = me.expression(selector)?;
                    let constant = plc_ir::Expression::constant(label_source, PlcValue::Int(value as i16));
                    Some(plc_ir::Expression::binary(label_source, PlcType::Bool, op, left, constant))
                };
                let test = if lo == hi {
                    compare(self, BinaryOperator::Equal, lo)?
                } else {
                    let low = compare(self, BinaryOperator::GreaterEqual, lo)?;
                    let high = compare(self, BinaryOperator::LessEqual, hi)?;
                    plc_ir::Expression::logical(label_source, BinaryOperator::And, vec![low, high])
                };
                tests.push(test);
            }
            let arm_source = self.anchor(arm.span);
            let condition = if tests.len() == 1 {
                tests.pop().expect("one test")
            } else {
                plc_ir::Expression::logical(arm_source, BinaryOperator::Or, tests)
            };
            let body = self.body(&arm.body);
            branches.push((arm_source, condition, body));
        }
        let mut tail = self.body(else_body);
        while let Some((arm_source, condition, body)) = branches.pop() {
            let statement_source = if branches.is_empty() { source } else { arm_source };
            tail = vec![plc_ir::Statement::conditional(statement_source, condition, body, tail)];
        }
        tail.pop()
    }
    fn expression(&mut self, expression: &ast::Expression) -> Option<plc_ir::Expression> {
        if self.errors.stopped {
            return None;
        }
        let source = self.anchor(expression.span);
        match &expression.kind {
            ast::ExpressionKind::Literal(literal) => self.literal(source, literal, false, expression.span),
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
                        format!("Unary {operator:?} cannot accept {}.", type_name(value.result_type())),
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
                let Some(kind) =
                    compatibility::binary_result(*operator, left_value.result_type(), right_value.result_type())
                else {
                    self.report(
                        codes::TYPE,
                        format!(
                            "{} does not accept {} and {}; operand types must follow the Alpha type rules.",
                            operator_name(*operator),
                            type_name(left_value.result_type()),
                            type_name(right_value.result_type())
                        ),
                        expression.span,
                    );
                    return None;
                };
                Some(plc_ir::Expression::binary(source, kind, *operator, left_value, right_value))
            }
            ast::ExpressionKind::Chain(operator, operands) => {
                let values: Vec<Option<plc_ir::Expression>> = operands.iter().map(|o| self.expression(o)).collect();
                let mut compiled = Vec::with_capacity(values.len());
                for (value, operand) in values.into_iter().zip(operands) {
                    let value = value?;
                    if value.result_type() != PlcType::Bool {
                        self.report(
                            codes::TYPE,
                            format!("{} requires BOOL operands, received {}.", operator_name(*operator), type_name(value.result_type())),
                            operand.span,
                        );
                        return None;
                    }
                    compiled.push(value);
                }
                Some(plc_ir::Expression::logical(source, *operator, compiled))
            }
            ast::ExpressionKind::Member(instance, member) => {
                let Some((id, kind)) = self.instances.get(&instance.text.to_ascii_uppercase()).copied() else {
                    self.report(
                        codes::UNKNOWN,
                        format!("'{}' is not a function-block instance; only block outputs can be read with '.'.", instance.text),
                        instance.span,
                    );
                    return None;
                };
                match kind.parameter_named(&member.text) {
                    Some(p) if p.direction == ParameterDirection::Output => Some(plc_ir::Expression::function_block_output(
                        source,
                        p.plc_type,
                        id,
                        ParameterId::new(p.id).expect("fixed parameter identity"),
                    )),
                    Some(p) => {
                        self.report(
                            codes::BLOCK,
                            format!("{}.{} is an input; only outputs can be read.", instance.text, p.name),
                            member.span,
                        );
                        None
                    }
                    None => {
                        let outputs: Vec<&str> = kind
                            .signature()
                            .iter()
                            .filter(|p| p.direction == ParameterDirection::Output)
                            .map(|p| p.name)
                            .collect();
                        self.report(
                            codes::BLOCK,
                            format!("{} has no output '{}'; use {}.", kind.name(), member.text, outputs.join(" or ")),
                            member.span,
                        );
                        None
                    }
                }
            }
            ast::ExpressionKind::Call(function, arguments) => {
                let Some(conversion) = ConversionFunction::from_name(&function.text) else {
                    self.report(
                        codes::UNSUPPORTED,
                        format!("Function '{}' is not available; Alpha provides INT_TO_REAL, REAL_TO_INT and TRUNC.", function.text),
                        function.span,
                    );
                    return None;
                };
                if arguments.len() != 1 {
                    self.report(codes::SYNTAX, format!("{} takes exactly one argument.", conversion.name()), expression.span);
                    return None;
                }
                let operand = self.expression(&arguments[0])?;
                let Some(kind) = compatibility::conversion_result(conversion, operand.result_type()) else {
                    self.report(
                        codes::TYPE,
                        format!("{} cannot convert {}.", conversion.name(), type_name(operand.result_type())),
                        arguments[0].span,
                    );
                    return None;
                };
                Some(plc_ir::Expression::conversion(source, kind, conversion, operand))
            }
        }
    }
    fn literal(&mut self, source: SourceRef, literal: &ast::Literal, negative: bool, span: TextSpan) -> Option<plc_ir::Expression> {
        match literals::value(literal, negative) {
            Ok(value) => Some(plc_ir::Expression::constant(source, value)),
            Err(message) => {
                self.report(codes::LITERAL, message, span);
                None
            }
        }
    }
    fn call(&mut self, name: &ast::Name, associations: &[ast::Association]) -> Option<FunctionBlockCall> {
        let Some((instance, kind)) = self.instances.get(&name.text.to_ascii_uppercase()).copied() else {
            self.report(
                codes::BLOCK,
                format!("Unknown function-block instance '{}'; declare it in VAR first.", name.text),
                name.span,
            );
            return None;
        };
        let before = self.errors.items.len();
        let mut seen = BTreeSet::new();
        let mut bindings = Vec::new();
        let names: Vec<&str> = kind.signature().iter().map(|p| p.name).collect();
        for association in associations {
            if self.errors.stopped {
                break;
            }
            let Some(parameter) = kind.parameter_named(&association.parameter.text) else {
                self.report(
                    codes::BLOCK,
                    format!(
                        "Unknown {} parameter '{}'; use {}.",
                        kind.name(),
                        association.parameter.text,
                        names.join(", ")
                    ),
                    association.parameter.span,
                );
                continue;
            };
            if !seen.insert(parameter.id) {
                self.report(
                    codes::BLOCK,
                    format!("{} parameter '{}' is associated more than once.", kind.name(), association.parameter.text),
                    association.parameter.span,
                );
                continue;
            }
            let id = ParameterId::new(parameter.id).expect("fixed parameter identity");
            match (&association.value, parameter.direction) {
                (ast::AssociationValue::Input(expression), ParameterDirection::Input) => {
                    if let Some(value) = self.expression(expression) {
                        if value.result_type() != parameter.plc_type {
                            self.report(
                                codes::BLOCK,
                                format!(
                                    "{} {} requires {}, received {}.",
                                    kind.name(),
                                    parameter.name,
                                    type_name(parameter.plc_type),
                                    type_name(value.result_type())
                                ),
                                expression.span,
                            );
                        } else {
                            bindings.push(CallBinding::input(id, value));
                        }
                    }
                }
                (ast::AssociationValue::Output(target), ParameterDirection::Output) => {
                    if let Some((symbol, target_type)) = self.target(target, codes::BLOCK) {
                        if target_type != parameter.plc_type {
                            self.report(
                                codes::BLOCK,
                                format!(
                                    "{} {} target must be {}, received {}.",
                                    kind.name(),
                                    parameter.name,
                                    type_name(parameter.plc_type),
                                    type_name(target_type)
                                ),
                                target.span,
                            );
                        } else {
                            bindings.push(CallBinding::output(id, symbol));
                        }
                    }
                }
                _ => self.report(
                    codes::BLOCK,
                    format!(
                        "Use {} for {} {}.",
                        if parameter.direction == ParameterDirection::Input { ":=" } else { "=>" },
                        kind.name(),
                        parameter.name
                    ),
                    association.span,
                ),
            }
        }
        for parameter in kind.signature().iter().filter(|p| p.required) {
            if !seen.contains(&parameter.id) {
                self.report(
                    codes::BLOCK,
                    format!("Required {} input {} is missing.", kind.name(), parameter.name),
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
                self.report(codes::BLOCK, "Block parameters must be associated at most once.", name.span);
                None
            }
        }
    }
}
fn signed_literal(expression: &ast::Expression) -> Option<&ast::Literal> {
    match &expression.kind {
        ast::ExpressionKind::Literal(literal @ (ast::Literal::Integer(_) | ast::Literal::Real(_))) => Some(literal),
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
fn operator_name(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::And => "AND",
        BinaryOperator::Or => "OR",
        BinaryOperator::Xor => "XOR",
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Modulo => "MOD",
        BinaryOperator::Equal => "=",
        BinaryOperator::NotEqual => "<>",
        BinaryOperator::Less => "<",
        BinaryOperator::LessEqual => "<=",
        BinaryOperator::Greater => ">",
        BinaryOperator::GreaterEqual => ">=",
    }
}
