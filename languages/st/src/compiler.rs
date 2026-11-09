use crate::{
    ast,
    diagnostics::{CompilerConfig, Diagnostics, codes, error},
    literals,
};
use plc_ir::{CallBinding, FunctionBlockCall, Program, ValidatedProgram};
use plc_types::{
    PlcType, PlcValue,
    compatibility::{
        self, AssignmentCompatibility, BinaryOperator, ConversionFunction, UnaryOperator,
    },
    diagnostics::Diagnostic,
    function_blocks::{
        FunctionBlockCatalog, FunctionBlockDefinition, FunctionBlockInstance, ParameterDirection,
        StandardFunctionBlock,
    },
    identity::{
        DocumentId, FunctionBlockId, FunctionBlockInstanceId, ParameterId, PouId, SourceElementId,
        SymbolId,
    },
    source::{SourceLanguage, SourceLocation, SourceMap, SourceRef, TextSpan},
    symbols::{
        Pou, PouKind, Retention, Scope, Symbol, SymbolTable, SystemVariable, VariableCategory,
    },
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationSummary {
    pub name: String,
    pub type_name: String,
    pub role: String,
    /// Declared in `VAR RETAIN` (ADR-0038).
    pub retained: bool,
    pub span: TextSpan,
}

/// A user FUNCTION or FUNCTION_BLOCK of the compiled document, for the library browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PouSummary {
    pub name: String,
    /// "FUNCTION" or "FUNCTION_BLOCK".
    pub kind: String,
    pub return_type: Option<String>,
    pub parameters: Vec<ParameterSummary>,
    /// The comment block right before the POU.
    pub description: Option<String>,
    pub span: TextSpan,
    /// 1 for the main program document, 2 and above for the library documents in the
    /// order they were compiled (ADR-0045).
    pub document: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSummary {
    pub name: String,
    pub type_name: String,
    /// "Input", "Output" or "InOut".
    pub direction: String,
    /// The comment after the declaration on its line.
    pub description: Option<String>,
}

/// An owned, immutable candidate that passed IR validation at construction.
/// Borrowed validation is explicit at the existing runtime-lowering boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledProgram {
    program: Program,
    name: String,
    declarations: Vec<DeclarationSummary>,
    pous: Vec<PouSummary>,
}
impl CompiledProgram {
    pub fn program_name(&self) -> &str {
        &self.name
    }
    pub fn declarations(&self) -> &[DeclarationSummary] {
        &self.declarations
    }
    /// The document's user functions and function blocks (ADR-0044).
    pub fn pous(&self) -> &[PouSummary] {
        &self.pous
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
    compile_st_documents(source, &[], configuration)
}

/// Library documents a project may have beside its main program (ADR-0045).
pub const MAX_LIBRARY_DOCUMENTS: usize = 31;

/// Compile the main program document together with the project library documents
/// (FUNCTIONs and FUNCTION_BLOCKs only) into one canonical program (ADR-0045). Source
/// anchors and diagnostics name their document: 1 for the main program, 2 and above for
/// `library` in order.
pub fn compile_st_documents(
    source: &str,
    library: &[&str],
    configuration: CompilerConfig,
) -> Result<CompiledProgram, Vec<Diagnostic>> {
    if library.len() > MAX_LIBRARY_DOCUMENTS {
        return Err(vec![crate::diagnostics::error(
            codes::LIMIT,
            format!("A project has at most {MAX_LIBRARY_DOCUMENTS} library documents."),
            None,
        )]);
    }
    let mut errors = Vec::new();
    let main = crate::parse_st(source, configuration)
        .map_err(|e| errors.extend(e))
        .ok();
    let mut pous = Vec::new();
    let mut texts = vec![source];
    for (index, text) in library.iter().enumerate() {
        let number = u32::try_from(index + 2).expect("bounded library documents");
        let document = DocumentId::new(number).expect("nonzero document number");
        texts.push(text);
        match crate::parse_st_library(text, configuration) {
            Ok(parsed) => pous.extend(parsed.into_iter().map(|mut pou| {
                pou.document = number;
                pou
            })),
            Err(found) => errors.extend(
                found
                    .into_iter()
                    .map(|d| crate::diagnostics::relocate(d, document)),
            ),
        }
    }
    let ast = match main {
        Some(mut ast) if errors.is_empty() => {
            ast.library.extend(pous);
            ast
        }
        _ => {
            errors.truncate(configuration.diagnostics);
            return Err(errors);
        }
    };
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
        library: ast.library.clone(),
        library_index: BTreeMap::new(),
        broken: vec![false; ast.library.len()],
        frame: Frame::default(),
        user_instances: BTreeMap::new(),
        pending: Vec::new(),
        expanding: Vec::new(),
        next_call: 0,
        emitted: 0,
        checking: false,
    };
    compiler
        .sources
        .register_document(document, SourceLanguage::St)
        .expect("fresh source registry");
    for number in 2..=texts.len() {
        let library = DocumentId::new(u32::try_from(number).expect("bounded library documents"))
            .expect("nonzero document number");
        compiler
            .sources
            .register_document(library, SourceLanguage::St)
            .expect("distinct library documents");
    }
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
    compiler.index_library(&ast.name);
    compiler.recursion();
    compiler.check_library();
    compiler.declare(&ast.declarations, false);
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
    let pous = ast
        .library
        .iter()
        .map(|pou| summary(texts[pou.document as usize - 1], pou))
        .collect();
    Ok(CompiledProgram {
        program,
        name: ast.name.text,
        declarations: compiler.declarations,
        pous,
    })
}

/// Nested function and function block expansion depth (ADR-0044).
const MAX_EXPANSION: usize = 64;
/// Statements a program may have after expansion.
const MAX_STATEMENTS: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum FrameKind {
    #[default]
    Program,
    FunctionBlock,
    Function,
}

/// The names visible while a POU body is compiled (ADR-0044).
#[derive(Debug, Clone, Default)]
struct Frame {
    kind: FrameKind,
    /// Prefix of the names declared for this POU: "" for the program, "Pump1." for an
    /// instance, "Scale#3." for one function call.
    prefix: String,
    /// The POU's declared roles by upper-case name.
    roles: BTreeMap<String, VariableCategory>,
    /// VAR_IN_OUT parameters bound to the caller's variables.
    aliases: BTreeMap<String, (SymbolId, PlcType, VariableCategory)>,
}
impl Frame {
    fn qualified(&self, name: &str) -> String {
        format!("{}{name}", self.prefix)
    }
    fn key(&self, name: &str) -> String {
        self.qualified(name).to_ascii_uppercase()
    }
}

fn roles(pou: &ast::PouAst) -> BTreeMap<String, VariableCategory> {
    let mut roles: BTreeMap<String, VariableCategory> = pou
        .declarations
        .iter()
        .map(|d| (d.name.text.to_ascii_uppercase(), d.role))
        .collect();
    if matches!(pou.kind, ast::PouKind::Function(_)) {
        roles.insert(pou.name.text.to_ascii_uppercase(), VariableCategory::Local);
    }
    roles
}

fn declared_type_name(kind: &ast::DeclaredType) -> String {
    match kind {
        ast::DeclaredType::Scalar(t) => type_name(*t).into(),
        ast::DeclaredType::FunctionBlock(k) => k.name().into(),
        ast::DeclaredType::Named(n) => n.text.clone(),
    }
}

/// The comment `(* ... *)` in `text`, trimmed.
fn comment(text: &str) -> Option<String> {
    let start = text.find("(*")?;
    let end = text[start..].find("*)")? + start;
    let inner = text[start + 2..end].trim();
    (!inner.is_empty()).then(|| inner.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn summary(source: &str, pou: &ast::PouAst) -> PouSummary {
    let lines: Vec<&str> = source.lines().collect();
    let line = |n: u32| {
        lines
            .get(n.saturating_sub(1) as usize)
            .copied()
            .unwrap_or("")
    };
    // The comment block that ends on the lines right before the POU keyword.
    let start = pou.span.start().line();
    let mut block = Vec::new();
    let mut n = start.saturating_sub(1);
    while n >= 1 && !line(n).trim().is_empty() && block.len() < 20 {
        block.insert(0, line(n));
        if line(n).trim_start().starts_with("(*") {
            break;
        }
        n -= 1;
    }
    let before = block.join(" ");
    let description = before
        .trim_end()
        .ends_with("*)")
        .then(|| comment(&before))
        .flatten();
    let parameters = pou
        .declarations
        .iter()
        .filter(|d| d.role != VariableCategory::Local)
        .map(|d| {
            let end = d.span.end();
            let rest: String = line(end.line())
                .chars()
                .skip(end.column().saturating_sub(1) as usize)
                .collect();
            ParameterSummary {
                name: d.name.text.clone(),
                type_name: declared_type_name(&d.kind),
                direction: match d.role {
                    VariableCategory::Input => "Input",
                    VariableCategory::Output => "Output",
                    _ => "InOut",
                }
                .into(),
                description: comment(&rest),
            }
        })
        .collect();
    PouSummary {
        name: pou.name.text.clone(),
        kind: match pou.kind {
            ast::PouKind::Function(_) => "FUNCTION",
            ast::PouKind::FunctionBlock => "FUNCTION_BLOCK",
        }
        .into(),
        return_type: match pou.kind {
            ast::PouKind::Function(t) => Some(type_name(t).into()),
            ast::PouKind::FunctionBlock => None,
        },
        parameters,
        description,
        span: pou.span,
        document: pou.document,
    }
}

/// Every user POU name a body or declaration list uses: called functions and declared
/// function block types.
fn uses(pou: &ast::PouAst) -> Vec<(String, TextSpan)> {
    fn expression(e: &ast::Expression, out: &mut Vec<(String, TextSpan)>) {
        match &e.kind {
            ast::ExpressionKind::Call(name, arguments) => {
                out.push((name.text.to_ascii_uppercase(), name.span));
                arguments.iter().for_each(|a| expression(a, out));
            }
            ast::ExpressionKind::Parentheses(x) | ast::ExpressionKind::Unary(_, x) => {
                expression(x, out)
            }
            ast::ExpressionKind::Binary(_, a, b) => {
                expression(a, out);
                expression(b, out);
            }
            ast::ExpressionKind::Chain(_, xs) => xs.iter().for_each(|x| expression(x, out)),
            _ => {}
        }
    }
    fn statements(body: &[ast::Statement], out: &mut Vec<(String, TextSpan)>) {
        for s in body {
            match &s.kind {
                ast::StatementKind::Assignment(_, e) => expression(e, out),
                ast::StatementKind::If(branches, else_body) => {
                    for b in branches {
                        expression(&b.condition, out);
                        statements(&b.body, out);
                    }
                    statements(else_body, out);
                }
                ast::StatementKind::Case(selector, arms, else_body) => {
                    expression(selector, out);
                    arms.iter().for_each(|a| statements(&a.body, out));
                    statements(else_body, out);
                }
                ast::StatementKind::Call(_, associations) => {
                    for a in associations {
                        if let ast::AssociationValue::Input(e) = &a.value {
                            expression(e, out);
                        }
                    }
                }
            }
        }
    }
    let mut out: Vec<(String, TextSpan)> = pou
        .declarations
        .iter()
        .filter_map(|d| match &d.kind {
            ast::DeclaredType::Named(n) => Some((n.text.to_ascii_uppercase(), n.span)),
            _ => None,
        })
        .collect();
    statements(&pou.body, &mut out);
    out
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
    /// User FUNCTIONs and FUNCTION_BLOCKs of the document (ADR-0044).
    library: Vec<ast::PouAst>,
    library_index: BTreeMap<String, usize>,
    /// POUs whose own errors are reported: they are not expanded.
    broken: Vec<bool>,
    frame: Frame,
    /// User function block instances by upper-case path.
    user_instances: BTreeMap<String, usize>,
    /// Expanded function code that must run before the statement being compiled.
    pending: Vec<plc_ir::Statement>,
    expanding: Vec<usize>,
    next_call: u32,
    emitted: usize,
    /// Checking a POU on its own: other POUs' bodies are not expanded.
    checking: bool,
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
    /// Declare a POU's variables in the current frame. `check` declares VAR_IN_OUT
    /// parameters as variables, so a body can be checked without a caller.
    fn declare(&mut self, declarations: &[ast::Declaration], check: bool) {
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
            if let Some(system) = SystemVariable::named(&name.text) {
                self.report(
                    codes::DUPLICATE,
                    format!(
                        "'{}' is reserved: the PLC supplies it as a read-only system variable.",
                        system.name()
                    ),
                    name.span,
                );
                continue;
            }
            let frame = self.frame.kind;
            let role = declaration.role;
            let rule = match (frame, role, &declaration.kind) {
                (FrameKind::Program, VariableCategory::InOut, _) => Some(
                    "VAR_IN_OUT belongs to function blocks; a PROGRAM has VAR_INPUT, VAR_OUTPUT and VAR.",
                ),
                (FrameKind::Function, VariableCategory::Output | VariableCategory::InOut, _) => {
                    Some(
                        "A function returns its value through its own name: it has VAR_INPUT and VAR only.",
                    )
                }
                (
                    FrameKind::Function,
                    _,
                    ast::DeclaredType::FunctionBlock(_) | ast::DeclaredType::Named(_),
                ) => Some(
                    "A function has no memory: declare function block instances in a FUNCTION_BLOCK or the PROGRAM.",
                ),
                (FrameKind::Function | FrameKind::FunctionBlock, _, _) if declaration.retain => {
                    Some("RETAIN is supported for PROGRAM variables only in PLC Lab.")
                }
                (_, VariableCategory::InOut, _) if declaration.initializer.is_some() => Some(
                    "A VAR_IN_OUT parameter is the caller's variable: it has no initial value.",
                ),
                _ => None,
            };
            if let Some(message) = rule {
                self.report(codes::UNSUPPORTED, message, declaration.span);
                continue;
            }
            let qualified = self.frame.qualified(&name.text);
            let category = if frame == FrameKind::Program {
                role
            } else {
                VariableCategory::Local
            };
            let source = self.anchor(declaration.span);
            match &declaration.kind {
                ast::DeclaredType::Scalar(_) if role == VariableCategory::InOut && !check => {}
                ast::DeclaredType::Scalar(kind) => {
                    let kind = *kind;
                    let initial = match &declaration.initializer {
                        None => None,
                        Some(_) if declaration.role == VariableCategory::Input => {
                            self.report(
                                codes::UNSUPPORTED,
                                "VAR_INPUT values come from the input image; an initial value is not allowed.",
                                declaration.span,
                            );
                            continue;
                        }
                        Some(expression) => {
                            match self.initial_value(&name.text, kind, expression) {
                                Some(value) => Some(value),
                                None => continue,
                            }
                        }
                    };
                    let id = SymbolId::new(self.next_symbol).expect("bounded declaration count");
                    self.next_symbol += 1;
                    let retention = if declaration.retain {
                        Retention::Retained
                    } else {
                        Retention::NonRetained
                    };
                    match Symbol::new(
                        id,
                        &qualified,
                        kind,
                        Scope::Pou(self.owner),
                        category,
                        Some(source),
                    )
                    .and_then(|s| s.with_retention(retention))
                    .and_then(|s| match initial {
                        Some(value) => s.with_initial(value),
                        None => Ok(s),
                    })
                    .and_then(|s| self.symbols.declare(s))
                    {
                        Ok(()) if frame != FrameKind::Program => {}
                        Ok(()) => self.declarations.push(DeclarationSummary {
                            name: name.text.clone(),
                            type_name: type_name(kind).into(),
                            role: match declaration.role {
                                VariableCategory::Input => "Input",
                                VariableCategory::Output => "Output",
                                _ => "Local",
                            }
                            .into(),
                            retained: declaration.retain,
                            span: declaration.span,
                        }),
                        Err(e) => self.errors.push(
                            e.diagnostic()
                                .with_source(self.sources.get(source).cloned()),
                        ),
                    }
                }
                ast::DeclaredType::Named(type_name) => {
                    self.declare_user_instance(declaration, type_name, &qualified);
                }
                ast::DeclaredType::FunctionBlock(kind) => {
                    let kind = *kind;
                    if declaration.retain {
                        self.report(
                            codes::UNSUPPORTED,
                            format!(
                                "{} instances are not retentive in PLC Lab: keep the value to retain in a VAR RETAIN variable, not the block instance.",
                                kind.name()
                            ),
                            declaration.span,
                        );
                        continue;
                    }
                    if declaration.initializer.is_some() {
                        self.report(
                            codes::UNSUPPORTED,
                            format!("A {} instance has no initial value.", kind.name()),
                            declaration.span,
                        );
                        continue;
                    }
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
                        let id = FunctionBlockId::new(
                            u32::try_from(self.definitions.len() + 1).unwrap_or(u32::MAX),
                        )
                        .expect("nonzero definition identity");
                        self.catalog
                            .declare_definition(
                                FunctionBlockDefinition::standard(id, kind).with_source(source),
                            )
                            .expect("one definition per standard kind");
                        self.definitions.insert(kind, id);
                        id
                    };
                    let id = FunctionBlockInstanceId::new(self.next_instance)
                        .expect("bounded instance count");
                    self.next_instance += 1;
                    match FunctionBlockInstance::new(
                        id,
                        definition,
                        self.owner,
                        &qualified,
                        Some(source),
                    )
                    .and_then(|i| self.catalog.declare_instance(i))
                    {
                        Ok(()) if frame != FrameKind::Program => {
                            self.instances
                                .insert(qualified.to_ascii_uppercase(), (id, kind));
                        }
                        Ok(()) => {
                            self.instances
                                .insert(name.text.to_ascii_uppercase(), (id, kind));
                            self.declarations.push(DeclarationSummary {
                                name: name.text.clone(),
                                type_name: kind.name().into(),
                                role: "FB Instance".into(),
                                retained: false,
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
    /// Run `work` in the document that defines library POU `index`, so its source
    /// anchors and diagnostics name that document (ADR-0045).
    fn in_document<T>(&mut self, index: usize, work: impl FnOnce(&mut Self) -> T) -> T {
        let document =
            DocumentId::new(self.library[index].document).expect("registered library document");
        let saved = std::mem::replace(&mut self.document, document);
        let result = work(self);
        self.document = saved;
        result
    }
    /// Index the document's POUs by name and refuse clashes.
    fn index_library(&mut self, program: &ast::Name) {
        for index in 0..self.library.len() {
            let name = self.library[index].name.clone();
            let upper = name.text.to_ascii_uppercase();
            let reserved = upper == program.text.to_ascii_uppercase()
                || ConversionFunction::from_name(&name.text).is_some()
                || SystemVariable::named(&name.text).is_some();
            if reserved || self.library_index.contains_key(&upper) {
                self.in_document(index, |c| {
                    c.report(
                        codes::DUPLICATE,
                        format!(
                            "'{}' is already the name of the program, another POU or a built-in function.",
                            name.text
                        ),
                        name.span,
                    )
                });
                self.broken[index] = true;
                continue;
            }
            self.library_index.insert(upper, index);
        }
    }
    /// Recursion, direct or through other POUs, is a compile error (ADR-0044).
    fn recursion(&mut self) {
        let edges: Vec<Vec<(usize, TextSpan)>> = self
            .library
            .iter()
            .map(|pou| {
                uses(pou)
                    .into_iter()
                    .filter_map(|(name, span)| self.library_index.get(&name).map(|i| (*i, span)))
                    .collect()
            })
            .collect();
        for start in 0..self.library.len() {
            // Depth-first search for a path back to `start`.
            let mut stack = vec![(start, 0usize)];
            let mut seen = BTreeSet::new();
            let mut found = None;
            while let Some((node, next)) = stack.pop() {
                let Some((target, span)) = edges[node].get(next).copied() else {
                    continue;
                };
                stack.push((node, next + 1));
                if target == start {
                    found = Some((node, span));
                    break;
                }
                if seen.insert(target) {
                    stack.push((target, 0));
                }
            }
            if let Some((via, span)) = found {
                let name = self.library[start].name.text.clone();
                let through = self.library[via].name.text.clone();
                let message = if via == start {
                    format!("'{name}' uses itself; recursion is not supported.")
                } else {
                    format!("'{name}' uses itself through '{through}'; recursion is not supported.")
                };
                self.in_document(via, |c| c.report(codes::UNSUPPORTED, message, span));
                self.broken[start] = true;
            }
        }
    }
    /// Compile every POU once on its own, so its errors are reported once at its
    /// definition, also when nothing calls it. The scratch symbols are discarded.
    fn check_library(&mut self) {
        for index in 0..self.library.len() {
            if self.broken[index] || self.errors.stopped {
                continue;
            }
            let pou = self.library[index].clone();
            let snapshot = (
                self.symbols.clone(),
                self.catalog.clone(),
                self.definitions.clone(),
                self.instances.clone(),
                self.user_instances.clone(),
                self.next_symbol,
                self.next_instance,
            );
            let before = self.errors.items.len();
            let kind = match pou.kind {
                ast::PouKind::Function(_) => FrameKind::Function,
                ast::PouKind::FunctionBlock => FrameKind::FunctionBlock,
            };
            let saved = std::mem::replace(
                &mut self.frame,
                Frame {
                    kind,
                    prefix: format!("{}#0.", pou.name.text),
                    roles: roles(&pou),
                    aliases: BTreeMap::new(),
                },
            );
            self.checking = true;
            self.expanding.push(index);
            self.in_document(index, |c| {
                c.declare(&pou.declarations, true);
                if let ast::PouKind::Function(returns) = pou.kind {
                    let qualified = c.frame.qualified(&pou.name.text);
                    c.temp(&qualified, returns);
                }
                let _ = c.body(&pou.body);
            });
            self.expanding.pop();
            self.checking = false;
            self.frame = saved;
            (
                self.symbols,
                self.catalog,
                self.definitions,
                self.instances,
                self.user_instances,
                self.next_symbol,
                self.next_instance,
            ) = snapshot;
            self.pending.clear();
            if self.errors.items.len() != before {
                self.broken[index] = true;
            }
        }
    }
    /// A compiler-generated variable (instance member or call temporary).
    fn temp(&mut self, name: &str, kind: PlcType) -> Option<SymbolId> {
        let id = SymbolId::new(self.next_symbol).expect("bounded declaration count");
        self.next_symbol += 1;
        let symbol = Symbol::new(
            id,
            name,
            kind,
            Scope::Pou(self.owner),
            VariableCategory::Local,
            None,
        )
        .ok()?;
        self.symbols.declare(symbol).ok()?;
        Some(id)
    }
    fn member(&self, path: &str) -> Option<SymbolId> {
        self.symbols
            .resolve(Scope::Pou(self.owner), path)
            .ok()
            .map(|s| s.id())
    }
    /// An instance of a user function block: one variable per member (instance memory).
    fn declare_user_instance(
        &mut self,
        declaration: &ast::Declaration,
        type_name: &ast::Name,
        qualified: &str,
    ) {
        let Some(index) = self
            .library_index
            .get(&type_name.text.to_ascii_uppercase())
            .copied()
        else {
            self.report(
                codes::UNSUPPORTED,
                format!(
                    "Type '{}' is unknown; use BOOL, INT, REAL, TIME, a standard block (TON, TOF, TP, R_TRIG, F_TRIG, CTU, CTD, PID) or a FUNCTION_BLOCK of this program.",
                    type_name.text
                ),
                type_name.span,
            );
            return;
        };
        let pou = self.library[index].clone();
        let problem = if matches!(pou.kind, ast::PouKind::Function(_)) {
            Some(format!(
                "'{}' is a FUNCTION: call it in an expression, it has no instances.",
                pou.name.text
            ))
        } else if declaration.role != VariableCategory::Local {
            Some(format!(
                "{} instances must be declared in VAR, not as parameters.",
                pou.name.text
            ))
        } else if declaration.retain {
            Some(format!(
                "{} instances are not retentive in PLC Lab: keep the value to retain in a VAR RETAIN variable.",
                pou.name.text
            ))
        } else if declaration.initializer.is_some() {
            Some(format!(
                "A {} instance has no initial value.",
                pou.name.text
            ))
        } else {
            None
        };
        if let Some(message) = problem {
            self.report(codes::BLOCK, message, declaration.span);
            return;
        }
        self.user_instances
            .insert(qualified.to_ascii_uppercase(), index);
        if self.frame.kind == FrameKind::Program {
            self.declarations.push(DeclarationSummary {
                name: declaration.name.text.clone(),
                type_name: pou.name.text.clone(),
                role: "FB Instance".into(),
                retained: false,
                span: declaration.span,
            });
        }
        if self.broken[index] {
            return;
        }
        let saved = std::mem::replace(
            &mut self.frame,
            Frame {
                kind: FrameKind::FunctionBlock,
                prefix: format!("{qualified}."),
                roles: roles(&pou),
                aliases: BTreeMap::new(),
            },
        );
        self.in_document(index, |c| c.declare(&pou.declarations, false));
        self.frame = saved;
    }
    /// Compile a POU body for one instance or call (ADR-0044).
    fn expand(
        &mut self,
        index: usize,
        kind: FrameKind,
        prefix: String,
        aliases: BTreeMap<String, (SymbolId, PlcType, VariableCategory)>,
        span: TextSpan,
    ) -> Option<Vec<plc_ir::Statement>> {
        if self.expanding.len() >= MAX_EXPANSION {
            self.report(
                codes::LIMIT,
                format!("More than {MAX_EXPANSION} nested function and function block calls."),
                span,
            );
            return None;
        }
        let pou = self.library[index].clone();
        let saved = std::mem::replace(
            &mut self.frame,
            Frame {
                kind,
                prefix,
                roles: roles(&pou),
                aliases,
            },
        );
        self.expanding.push(index);
        let body = self.in_document(index, |c| c.body(&pou.body));
        self.expanding.pop();
        self.frame = saved;
        Some(body)
    }
    fn note_emitted(&mut self, count: usize, span: TextSpan) -> Option<()> {
        self.emitted += count;
        if self.emitted > MAX_STATEMENTS {
            self.report(
                codes::LIMIT,
                format!("The program has more than {MAX_STATEMENTS} statements after expanding its function and function block calls."),
                span,
            );
            return None;
        }
        Some(())
    }
    /// A user function call: arguments, then the function body on fresh temporaries,
    /// hoisted before the current statement; the call reads the return temporary.
    fn function_call(
        &mut self,
        index: usize,
        function: &ast::Name,
        arguments: &[ast::Expression],
        source: SourceRef,
        span: TextSpan,
    ) -> Option<plc_ir::Expression> {
        let pou = self.library[index].clone();
        let ast::PouKind::Function(returns) = pou.kind else {
            self.report(
                codes::BLOCK,
                format!(
                    "'{}' is a FUNCTION_BLOCK: declare an instance in VAR and call it as a statement.",
                    pou.name.text
                ),
                function.span,
            );
            return None;
        };
        let inputs: Vec<&ast::Declaration> = pou
            .declarations
            .iter()
            .filter(|d| d.role == VariableCategory::Input)
            .collect();
        if arguments.len() != inputs.len() {
            let names: Vec<&str> = inputs.iter().map(|d| d.name.text.as_str()).collect();
            self.report(
                codes::BLOCK,
                format!(
                    "{} takes {} argument(s) in this order: {}.",
                    pou.name.text,
                    inputs.len(),
                    names.join(", ")
                ),
                span,
            );
            return None;
        }
        let mut values = Vec::new();
        let mut ok = true;
        for (argument, input) in arguments.iter().zip(&inputs) {
            let ast::DeclaredType::Scalar(kind) = input.kind else {
                ok = false;
                continue;
            };
            match self.expression(argument) {
                Some(value) if value.result_type() == kind => values.push(value),
                Some(value) => {
                    self.report(
                        codes::TYPE,
                        format!(
                            "{} argument {} requires {}, received {}.",
                            pou.name.text,
                            input.name.text,
                            type_name(kind),
                            type_name(value.result_type())
                        ),
                        argument.span,
                    );
                    ok = false;
                }
                None => ok = false,
            }
        }
        if !ok || self.broken[index] {
            return None;
        }
        if self.checking {
            // Checking another POU: the signature is enough.
            return Some(plc_ir::Expression::constant(
                source,
                returns.default_value(),
            ));
        }
        self.next_call += 1;
        let prefix = format!("{}#{}.", pou.name.text, self.next_call);
        let mut code = Vec::new();
        let result = self.temp(&format!("{prefix}{}", pou.name.text), returns)?;
        code.push(plc_ir::Statement::assign(
            source,
            result,
            plc_ir::Expression::constant(source, returns.default_value()),
        ));
        for local in pou
            .declarations
            .iter()
            .filter(|d| d.role == VariableCategory::Local)
        {
            let ast::DeclaredType::Scalar(kind) = local.kind else {
                continue;
            };
            let value = match &local.initializer {
                Some(e) => self.initial_value(&local.name.text, kind, e)?,
                None => kind.default_value(),
            };
            let id = self.temp(&format!("{prefix}{}", local.name.text), kind)?;
            code.push(plc_ir::Statement::assign(
                source,
                id,
                plc_ir::Expression::constant(source, value),
            ));
        }
        for (input, value) in inputs.iter().zip(values) {
            let kind = value.result_type();
            let id = self.temp(&format!("{prefix}{}", input.name.text), kind)?;
            code.push(plc_ir::Statement::assign(source, id, value));
        }
        let body = self.expand(index, FrameKind::Function, prefix, BTreeMap::new(), span)?;
        code.extend(body);
        self.note_emitted(code.len(), span)?;
        self.pending.extend(code);
        Some(plc_ir::Expression::read(source, result, returns))
    }
    /// A call of a user function block instance: inputs, the body on the instance's
    /// memory (VAR_IN_OUT on the caller's variables), then outputs.
    fn user_call(
        &mut self,
        name: &ast::Name,
        index: usize,
        associations: &[ast::Association],
        source: SourceRef,
    ) -> Option<Vec<plc_ir::Statement>> {
        let pou = self.library[index].clone();
        let path = self.frame.qualified(&name.text);
        let before = self.errors.items.len();
        let parameters: Vec<&ast::Declaration> = pou
            .declarations
            .iter()
            .filter(|d| d.role != VariableCategory::Local)
            .collect();
        let names: Vec<&str> = parameters.iter().map(|d| d.name.text.as_str()).collect();
        let mut seen = BTreeSet::new();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut aliases = BTreeMap::new();
        for association in associations {
            if self.errors.stopped {
                break;
            }
            let Some(parameter) = parameters.iter().find(|d| {
                d.name
                    .text
                    .eq_ignore_ascii_case(&association.parameter.text)
            }) else {
                self.report(
                    codes::BLOCK,
                    format!(
                        "Unknown {} parameter '{}'; use {}.",
                        pou.name.text,
                        association.parameter.text,
                        names.join(", ")
                    ),
                    association.parameter.span,
                );
                continue;
            };
            let upper = parameter.name.text.to_ascii_uppercase();
            if !seen.insert(upper.clone()) {
                self.report(
                    codes::BLOCK,
                    format!(
                        "{} parameter '{}' is associated more than once.",
                        pou.name.text, parameter.name.text
                    ),
                    association.parameter.span,
                );
                continue;
            }
            let ast::DeclaredType::Scalar(kind) = parameter.kind else {
                continue;
            };
            let member = format!("{path}.{}", parameter.name.text);
            match (&association.value, parameter.role) {
                (ast::AssociationValue::Input(expression), VariableCategory::Input) => {
                    if let Some(value) = self.expression(expression) {
                        if value.result_type() != kind {
                            self.report(
                                codes::BLOCK,
                                format!(
                                    "{} {} requires {}, received {}.",
                                    pou.name.text,
                                    parameter.name.text,
                                    type_name(kind),
                                    type_name(value.result_type())
                                ),
                                expression.span,
                            );
                        } else if let Some(symbol) = self.member(&member) {
                            inputs.push(plc_ir::Statement::assign(source, symbol, value));
                        }
                    }
                }
                (ast::AssociationValue::Input(expression), VariableCategory::InOut) => {
                    let ast::ExpressionKind::Identifier(variable) = &expression.kind else {
                        self.report(
                            codes::BLOCK,
                            format!(
                                "VAR_IN_OUT {} of {} needs a variable, not an expression.",
                                parameter.name.text, pou.name.text
                            ),
                            expression.span,
                        );
                        continue;
                    };
                    if let Some((symbol, actual)) = self.target(variable, codes::BLOCK) {
                        if actual != kind {
                            self.report(
                                codes::BLOCK,
                                format!(
                                    "VAR_IN_OUT {} of {} is {}, '{}' is {}.",
                                    parameter.name.text,
                                    pou.name.text,
                                    type_name(kind),
                                    variable.text,
                                    type_name(actual)
                                ),
                                variable.span,
                            );
                        } else {
                            aliases.insert(upper, (symbol, kind, VariableCategory::Local));
                        }
                    }
                }
                (ast::AssociationValue::Output(target), VariableCategory::Output) => {
                    if let Some((symbol, actual)) = self.target(target, codes::BLOCK) {
                        if actual != kind {
                            self.report(
                                codes::BLOCK,
                                format!(
                                    "{} {} target must be {}, received {}.",
                                    pou.name.text,
                                    parameter.name.text,
                                    type_name(kind),
                                    type_name(actual)
                                ),
                                target.span,
                            );
                        } else if let Some(from) = self.member(&member) {
                            outputs.push(plc_ir::Statement::assign(
                                source,
                                symbol,
                                plc_ir::Expression::read(source, from, kind),
                            ));
                        }
                    }
                }
                _ => self.report(
                    codes::BLOCK,
                    format!(
                        "Use := for {} inputs and VAR_IN_OUT parameters, => for outputs ({}).",
                        pou.name.text, parameter.name.text
                    ),
                    association.span,
                ),
            }
        }
        for parameter in parameters
            .iter()
            .filter(|d| d.role == VariableCategory::InOut)
        {
            if !seen.contains(&parameter.name.text.to_ascii_uppercase()) {
                self.report(
                    codes::BLOCK,
                    format!(
                        "VAR_IN_OUT {} of {} must be bound to a variable.",
                        parameter.name.text, pou.name.text
                    ),
                    name.span,
                );
            }
        }
        if self.errors.items.len() != before {
            return None;
        }
        if self.broken[index] || self.checking {
            inputs.extend(outputs);
            return Some(inputs);
        }
        let body = self.expand(
            index,
            FrameKind::FunctionBlock,
            format!("{path}."),
            aliases,
            name.span,
        )?;
        let mut code = inputs;
        code.extend(body);
        code.extend(outputs);
        self.note_emitted(code.len(), name.span)?;
        Some(code)
    }
    /// The output of a user function block instance.
    fn user_output(
        &mut self,
        instance: &ast::Name,
        member: &ast::Name,
        index: usize,
        source: SourceRef,
    ) -> Option<plc_ir::Expression> {
        let pou = self.library[index].clone();
        match pou
            .declarations
            .iter()
            .find(|d| d.name.text.eq_ignore_ascii_case(&member.text))
        {
            Some(d) if d.role == VariableCategory::Output => {
                let path = format!("{}.{}", self.frame.qualified(&instance.text), d.name.text);
                let symbol = self.symbols.resolve(Scope::Pou(self.owner), &path).ok()?;
                Some(plc_ir::Expression::read(
                    source,
                    symbol.id(),
                    symbol.plc_type(),
                ))
            }
            found => {
                let outputs: Vec<&str> = pou
                    .declarations
                    .iter()
                    .filter(|d| d.role == VariableCategory::Output)
                    .map(|d| d.name.text.as_str())
                    .collect();
                self.report(
                    codes::BLOCK,
                    if found.is_some() {
                        format!(
                            "{}.{} is not an output of {}; only VAR_OUTPUT values can be read.",
                            instance.text, member.text, pou.name.text
                        )
                    } else {
                        format!(
                            "{} has no output '{}'; use {}.",
                            pou.name.text,
                            member.text,
                            outputs.join(" or ")
                        )
                    },
                    member.span,
                );
                None
            }
        }
    }
    /// A constant initializer of exactly the declared type.
    fn initial_value(
        &mut self,
        name: &str,
        kind: PlcType,
        expression: &ast::Expression,
    ) -> Option<PlcValue> {
        let (literal, negative) = match &expression.kind {
            ast::ExpressionKind::Literal(literal) => (literal, false),
            ast::ExpressionKind::Parentheses(inner) => {
                return self.initial_value(name, kind, inner);
            }
            ast::ExpressionKind::Unary(UnaryOperator::Negate, inner)
                if signed_literal(inner).is_some() =>
            {
                (signed_literal(inner)?, true)
            }
            _ => {
                self.report(
                    codes::UNSUPPORTED,
                    format!(
                        "The initial value of '{name}' must be a constant {} literal.",
                        type_name(kind)
                    ),
                    expression.span,
                );
                return None;
            }
        };
        let value = match literals::value(literal, negative) {
            Ok(value) => value,
            Err(message) => {
                self.report(codes::LITERAL, message, expression.span);
                return None;
            }
        };
        if value.plc_type() != kind {
            self.report(
                codes::TYPE,
                format!(
                    "The initial value of '{name}' must be {}, not {}.",
                    type_name(kind),
                    type_name(value.plc_type())
                ),
                expression.span,
            );
            return None;
        }
        Some(value)
    }
    /// The program's view of a system variable, declared on first use.
    fn system_variable(&mut self, variable: SystemVariable) -> Option<(SymbolId, PlcType)> {
        if let Ok(symbol) = self
            .symbols
            .resolve(Scope::Pou(self.owner), variable.name())
        {
            return Some((symbol.id(), symbol.plc_type()));
        }
        let id = SymbolId::new(self.next_symbol).expect("bounded declaration count");
        self.next_symbol += 1;
        let symbol = Symbol::system_variable(id, variable, Scope::Pou(self.owner)).ok()?;
        self.symbols.declare(symbol).ok()?;
        Some((id, variable.plc_type()))
    }
    fn resolve(&mut self, name: &ast::Name) -> Option<(SymbolId, PlcType, VariableCategory)> {
        if let Some(variable) = SystemVariable::named(&name.text) {
            let (id, kind) = self.system_variable(variable)?;
            return Some((id, kind, VariableCategory::Local));
        }
        let upper = name.text.to_ascii_uppercase();
        if let Some(alias) = self.frame.aliases.get(&upper) {
            return Some(*alias);
        }
        let qualified = self.frame.qualified(&name.text);
        match self.symbols.resolve(Scope::Pou(self.owner), &qualified) {
            Ok(symbol) => {
                let role = self
                    .frame
                    .roles
                    .get(&upper)
                    .copied()
                    .unwrap_or(symbol.category());
                Some((symbol.id(), symbol.plc_type(), role))
            }
            Err(_) => {
                let key = self.frame.key(&name.text);
                let message = if self.instances.contains_key(&key)
                    || self.user_instances.contains_key(&key)
                {
                    format!(
                        "'{}' is a function-block instance; read one of its outputs, for example {}.Q.",
                        name.text, name.text
                    )
                } else {
                    format!("Unknown variable '{}'.", name.text)
                };
                self.report(codes::UNKNOWN, message, name.span);
                None
            }
        }
    }
    fn target(&mut self, name: &ast::Name, code: &str) -> Option<(SymbolId, PlcType)> {
        if let Some(variable) = SystemVariable::named(&name.text) {
            self.report(
                code,
                format!(
                    "Cannot write {}: the PLC supplies it and it is read-only.",
                    variable.name()
                ),
                name.span,
            );
            return None;
        }
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
            // Function code the statement needs runs right before it (ADR-0044).
            let outer = std::mem::take(&mut self.pending);
            let compiled = self.statement(statement);
            let hoisted = std::mem::replace(&mut self.pending, outer);
            if let Some(compiled) = compiled {
                body.extend(hoisted);
                body.extend(compiled);
            }
        }
        body
    }
    fn condition(
        &mut self,
        expression: &ast::Expression,
        what: &str,
    ) -> Option<plc_ir::Expression> {
        let value = self.expression(expression)?;
        if value.result_type() != PlcType::Bool {
            self.report(
                codes::TYPE,
                format!("{what} must be BOOL."),
                expression.span,
            );
            return None;
        }
        Some(value)
    }
    fn statement(&mut self, statement: &ast::Statement) -> Option<Vec<plc_ir::Statement>> {
        let source = self.anchor(statement.span);
        if !self.checking {
            self.note_emitted(1, statement.span)?;
        }
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
                            "Cannot assign {} to {} '{}'; PLC Lab Structured Text has no implicit conversions. Use INT_TO_REAL, REAL_TO_INT or TRUNC.",
                            type_name(value.result_type()),
                            type_name(target_type),
                            name.text
                        ),
                        expression.span,
                    );
                    return None;
                }
                Some(vec![plc_ir::Statement::assign(source, target, value)])
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
                    tail = vec![plc_ir::Statement::conditional(
                        branch_source,
                        condition,
                        body,
                        tail,
                    )];
                }
                Some(vec![plc_ir::Statement::conditional(
                    source,
                    first_condition,
                    first_body,
                    tail,
                )])
            }
            ast::StatementKind::Case(selector, arms, else_body) => self
                .case(source, selector, arms, else_body)
                .map(|statement| vec![statement]),
            ast::StatementKind::Call(name, associations) => {
                let key = self.frame.key(&name.text);
                if let Some(index) = self.user_instances.get(&key).copied() {
                    self.user_call(name, index, associations, source)
                } else {
                    self.call(name, associations)
                        .map(|call| vec![plc_ir::Statement::function_block_call(source, call)])
                }
            }
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
            self.report(
                codes::TYPE,
                "The CASE selector must be an INT expression.",
                selector.span,
            );
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
                    self.report(
                        codes::LITERAL,
                        "CASE label range must be ascending INT values.",
                        span,
                    );
                    return None;
                }
                if seen.iter().any(|(a, b)| lo <= *b && *a <= hi) {
                    self.report(codes::DUPLICATE, "CASE labels must not overlap.", span);
                    return None;
                }
                seen.push((lo, hi));
                let label_source = self.anchor(span);
                let compare =
                    |me: &mut Self, op: BinaryOperator, value: i64| -> Option<plc_ir::Expression> {
                        let left = me.expression(selector)?;
                        let constant =
                            plc_ir::Expression::constant(label_source, PlcValue::Int(value as i16));
                        Some(plc_ir::Expression::binary(
                            label_source,
                            PlcType::Bool,
                            op,
                            left,
                            constant,
                        ))
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
            let statement_source = if branches.is_empty() {
                source
            } else {
                arm_source
            };
            tail = vec![plc_ir::Statement::conditional(
                statement_source,
                condition,
                body,
                tail,
            )];
        }
        tail.pop()
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
                    self.report(
                        codes::TYPE,
                        format!(
                            "{} does not accept {} and {}; both operands need compatible types (convert explicitly with INT_TO_REAL, REAL_TO_INT or TRUNC).",
                            operator_name(*operator),
                            type_name(left_value.result_type()),
                            type_name(right_value.result_type())
                        ),
                        expression.span,
                    );
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
            ast::ExpressionKind::Chain(operator, operands) => {
                let values: Vec<Option<plc_ir::Expression>> =
                    operands.iter().map(|o| self.expression(o)).collect();
                let mut compiled = Vec::with_capacity(values.len());
                for (value, operand) in values.into_iter().zip(operands) {
                    let value = value?;
                    if value.result_type() != PlcType::Bool {
                        self.report(
                            codes::TYPE,
                            format!(
                                "{} requires BOOL operands, received {}.",
                                operator_name(*operator),
                                type_name(value.result_type())
                            ),
                            operand.span,
                        );
                        return None;
                    }
                    compiled.push(value);
                }
                Some(plc_ir::Expression::logical(source, *operator, compiled))
            }
            ast::ExpressionKind::Member(instance, member) => {
                let key = self.frame.key(&instance.text);
                if let Some(index) = self.user_instances.get(&key).copied() {
                    return self.user_output(instance, member, index, source);
                }
                let Some((id, kind)) = self.instances.get(&key).copied() else {
                    self.report(
                        codes::UNKNOWN,
                        format!("'{}' is not a function-block instance; only block outputs can be read with '.'.", instance.text),
                        instance.span,
                    );
                    return None;
                };
                match kind.parameter_named(&member.text) {
                    Some(p) if p.direction == ParameterDirection::Output => {
                        Some(plc_ir::Expression::function_block_output(
                            source,
                            p.plc_type,
                            id,
                            ParameterId::new(p.id).expect("fixed parameter identity"),
                        ))
                    }
                    Some(p) => {
                        self.report(
                            codes::BLOCK,
                            format!(
                                "{}.{} is an input; only outputs can be read.",
                                instance.text, p.name
                            ),
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
                            format!(
                                "{} has no output '{}'; use {}.",
                                kind.name(),
                                member.text,
                                outputs.join(" or ")
                            ),
                            member.span,
                        );
                        None
                    }
                }
            }
            ast::ExpressionKind::Call(function, arguments) => {
                if let Some(index) = self
                    .library_index
                    .get(&function.text.to_ascii_uppercase())
                    .copied()
                {
                    return self.function_call(index, function, arguments, source, expression.span);
                }
                let Some(conversion) = ConversionFunction::from_name(&function.text) else {
                    self.report(
                        codes::UNSUPPORTED,
                        format!("Function '{}' is not available; use INT_TO_REAL, REAL_TO_INT, TRUNC or a FUNCTION of this program.", function.text),
                        function.span,
                    );
                    return None;
                };
                if arguments.len() != 1 {
                    self.report(
                        codes::SYNTAX,
                        format!("{} takes exactly one argument.", conversion.name()),
                        expression.span,
                    );
                    return None;
                }
                let operand = self.expression(&arguments[0])?;
                let Some(kind) =
                    compatibility::conversion_result(conversion, operand.result_type())
                else {
                    self.report(
                        codes::TYPE,
                        format!(
                            "{} cannot convert {}.",
                            conversion.name(),
                            type_name(operand.result_type())
                        ),
                        arguments[0].span,
                    );
                    return None;
                };
                Some(plc_ir::Expression::conversion(
                    source, kind, conversion, operand,
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
        let Some((instance, kind)) = self.instances.get(&self.frame.key(&name.text)).copied()
        else {
            self.report(
                codes::BLOCK,
                format!(
                    "Unknown function-block instance '{}'; declare it in VAR first.",
                    name.text
                ),
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
                    format!(
                        "{} parameter '{}' is associated more than once.",
                        kind.name(),
                        association.parameter.text
                    ),
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
                        if parameter.direction == ParameterDirection::Input {
                            ":="
                        } else {
                            "=>"
                        },
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
                    format!(
                        "Required {} input {} is missing.",
                        kind.name(),
                        parameter.name
                    ),
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
                    "Block parameters must be associated at most once.",
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
