//! Typed semantic IR and validation only. There is no evaluator or execution backend.

mod model;
mod validation;

pub use model::*;
pub use validation::{MAX_VALIDATION_DEPTH, MAX_VALIDATION_NODES, ValidatedProgram, validate};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
