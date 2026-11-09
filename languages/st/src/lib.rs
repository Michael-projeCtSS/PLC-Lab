//! Bounded Alpha ST frontend. Authoritative execution remains in plc-runtime.

pub mod ast;
mod compiler;
mod diagnostics;
pub mod lexer;
mod literals;
mod parser;

pub use compiler::{
    CompiledProgram, DeclarationSummary, MAX_LIBRARY_DOCUMENTS, ParameterSummary, PouSummary,
    compile_st, compile_st_documents,
};
pub use diagnostics::{CompilerConfig, codes};

pub fn parse_st(
    source: &str,
    configuration: CompilerConfig,
) -> Result<ast::ProgramAst, Vec<plc_types::diagnostics::Diagnostic>> {
    let tokens = lexer::lex(source, configuration)?;
    parser::parse(&tokens, configuration)
}

/// Parse a project library document: FUNCTIONs and FUNCTION_BLOCKs only (ADR-0045).
pub fn parse_st_library(
    source: &str,
    configuration: CompilerConfig,
) -> Result<Vec<ast::PouAst>, Vec<plc_types::diagnostics::Diagnostic>> {
    let tokens = lexer::lex(source, configuration)?;
    parser::parse_library(&tokens, configuration)
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
