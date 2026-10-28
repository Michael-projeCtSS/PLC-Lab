use plc_types::{
    diagnostics::{Diagnostic, DiagnosticCategory, DiagnosticCode, Severity},
    identity::{DocumentId, SourceElementId},
    source::{SourceLanguage, SourceLocation, SourceRef, TextSpan},
};

pub mod codes {
    pub const LEXICAL: &str = "PLC-ST-001";
    pub const SYNTAX: &str = "PLC-ST-002";
    pub const LITERAL: &str = "PLC-ST-003";
    pub const UNSUPPORTED: &str = "PLC-ST-004";
    pub const DUPLICATE: &str = "PLC-ST-005";
    pub const UNKNOWN: &str = "PLC-ST-006";
    pub const TYPE: &str = "PLC-ST-007";
    pub const TARGET: &str = "PLC-ST-008";
    pub const BLOCK: &str = "PLC-ST-009";
    pub const LIMIT: &str = "PLC-ST-010";
}

/// Nonzero caller limits with absolute ceilings protecting parser stack/allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerConfig {
    pub(crate) source_bytes: usize,
    pub(crate) tokens: usize,
    pub(crate) nesting: usize,
    pub(crate) diagnostics: usize,
}
impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            source_bytes: 65_536,
            tokens: 16_384,
            nesting: 32,
            diagnostics: 32,
        }
    }
}
impl CompilerConfig {
    pub fn new(
        source_bytes: usize,
        tokens: usize,
        nesting: usize,
        diagnostics: usize,
    ) -> Result<Self, Diagnostic> {
        if !(1..=1_048_576).contains(&source_bytes)
            || !(1..=65_536).contains(&tokens)
            || !(1..=48).contains(&nesting)
            || !(1..=128).contains(&diagnostics)
        {
            return Err(error(
                codes::LIMIT,
                "Compiler limits must be nonzero and within the documented PLC Lab limits.",
                None,
            ));
        }
        Ok(Self {
            source_bytes,
            tokens,
            nesting,
            diagnostics,
        })
    }
}

pub(crate) fn location(span: TextSpan, element: u32) -> SourceLocation {
    SourceLocation::new(
        SourceRef::new(
            DocumentId::new(1).expect("fixed document"),
            SourceElementId::new(element).expect("bounded nonzero source identity"),
        ),
        SourceLanguage::St,
        Some(span),
    )
}
pub(crate) fn error(code: &str, message: impl Into<String>, span: Option<TextSpan>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("fixed ST code"),
        DiagnosticCategory::CompilerDiagnostic,
        Severity::Error,
        message,
    )
    .expect("nonempty ST explanation")
    .with_source(span.map(|s| location(s, 1)))
}

pub(crate) struct Diagnostics {
    pub(crate) items: Vec<Diagnostic>,
    limit: usize,
    pub(crate) stopped: bool,
}
impl Diagnostics {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            items: Vec::new(),
            limit,
            stopped: false,
        }
    }
    pub(crate) fn push(&mut self, diagnostic: Diagnostic) {
        if self.stopped {
            return;
        }
        if self.items.len() + 1 == self.limit {
            let source = diagnostic.source().cloned();
            self.items.push(
                error(
                    codes::LIMIT,
                    "Diagnostic limit reached; fix the reported errors and compile again.",
                    None,
                )
                .with_source(source),
            );
            self.stopped = true;
        } else {
            self.items.push(diagnostic);
        }
    }
}
