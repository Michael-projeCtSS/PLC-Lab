//! One headless Alpha task over an owned execution plan. No languages, blocks, or UI.

mod errors;
mod executor;
mod function_blocks;
mod memory;
mod operators;
mod plan;
mod runtime;

pub use errors::{
    ConfigurationError, InputError, InternalError, LoweringError, RuntimeFault, ScanError,
};
pub use function_blocks::FunctionBlockStateSnapshot;
pub use plan::{ExecutionPlan, MemoryRole, lower};
pub use runtime::{Runtime, RuntimeConfig, RuntimeSnapshot, RuntimeState, ScanResult};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
