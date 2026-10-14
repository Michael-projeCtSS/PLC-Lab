//! Deterministic virtual clock. No process models, pacing, or runtime dependency.

mod clock;
pub use clock::{ClockOverflow, SimulationClock};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
