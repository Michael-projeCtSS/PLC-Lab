//! Deterministic simulation time: the PLC task clock and the world scheduler.
//! No process models, host pacing, wall clock or runtime dependency.

mod clock;
mod world;
pub use clock::{ClockOverflow, SimulationClock};
pub use world::{EventKey, Phase, ScheduleError, Scheduler, WorldTime, deterministic_hash};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
