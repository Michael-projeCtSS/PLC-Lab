//! Bounded Alpha ST frontend. Authoritative execution remains in plc-runtime.

pub mod ast;
mod compiler;
mod diagnostics;
pub mod lexer;
mod literals;
mod parser;

pub use compiler::{CompiledProgram, DeclarationSummary, compile_st};
pub use diagnostics::{CompilerConfig, codes};

pub fn parse_st(
    source: &str,
    configuration: CompilerConfig,
) -> Result<ast::ProgramAst, Vec<plc_types::diagnostics::Diagnostic>> {
    let tokens = lexer::lex(source, configuration)?;
    parser::parse(&tokens, configuration)
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
