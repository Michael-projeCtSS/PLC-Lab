//! Headless semantic vocabulary for the generic Alpha PLC profile.

pub mod compatibility;
pub mod diagnostics;
pub mod function_blocks;
pub mod identity;
pub mod source;
pub mod symbols;
pub mod values;

pub use values::{PlcReal, PlcTime, PlcType, PlcValue, ValueError};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
