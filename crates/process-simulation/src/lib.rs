//! Headless physical models. No PLC memory, runtime, host or rendering dependency.
use plc_types::{PlcReal, PlcTime};

pub const LOW_THRESHOLD_PERMILLE: u16 = 200;
pub const HIGH_THRESHOLD_PERMILLE: u16 = 800;
pub const FILL_RATE_PERMILLE_PER_SECOND: u16 = 1000;
pub const DRAIN_RATE_PERMILLE_PER_SECOND: u16 = 800;
const MAX_LEVEL: i128 = 1000;
const MICROS_PER_SECOND: i128 = 1_000_000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TankActuators {
    pub inlet_valve: bool,
    pub drain_pump: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TankSensors {
    pub low_level: bool,
    pub high_level: bool,
    pub level_percent: PlcReal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TankSnapshot {
    level_permille: u16,
    actuators: TankActuators,
    sensors: TankSensors,
}
impl TankSnapshot {
    pub fn level_permille(self) -> u16 {
        self.level_permille
    }
    pub fn actuators(self) -> TankActuators {
        self.actuators
    }
    pub fn sensors(self) -> TankSensors {
        self.sensors
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TankFillingProcess {
    level_permille: u16,
    applied: TankActuators,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessInvariantError;
impl std::fmt::Display for ProcessInvariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The physical model could not complete a consistent step.")
    }
}
impl std::error::Error for ProcessInvariantError {}

fn contribution(rate: u16, dt: PlcTime, enabled: bool) -> Result<i128, ProcessInvariantError> {
    if !enabled {
        return Ok(0);
    }
    i128::from(rate)
        .checked_mul(i128::from(dt.as_micros()))
        .map(|product| product / MICROS_PER_SECOND)
        .ok_or(ProcessInvariantError)
}
impl TankFillingProcess {
    /// Initialize an empty plant with actual previously committed commands.
    pub fn new(applied: TankActuators) -> Self {
        Self {
            level_permille: 0,
            applied,
        }
    }
    pub fn snapshot(&self) -> TankSnapshot {
        TankSnapshot {
            level_permille: self.level_permille,
            actuators: self.applied,
            sensors: TankSensors {
                low_level: self.level_permille <= LOW_THRESHOLD_PERMILLE,
                high_level: self.level_permille >= HIGH_THRESHOLD_PERMILLE,
                // Private bounded level guarantees a finite 0..100 representation.
                level_percent: PlcReal::new(f32::from(self.level_permille) / 10.0)
                    .expect("A bounded tank level is a finite analog sensor."),
            },
        }
    }
    pub fn step(
        &mut self,
        cycle: PlcTime,
        actuators: TankActuators,
    ) -> Result<TankSnapshot, ProcessInvariantError> {
        let inlet = contribution(FILL_RATE_PERMILLE_PER_SECOND, cycle, actuators.inlet_valve)?;
        let drain = contribution(DRAIN_RATE_PERMILLE_PER_SECOND, cycle, actuators.drain_pump)?;
        let next = i128::from(self.level_permille)
            .checked_add(inlet)
            .and_then(|level| level.checked_sub(drain))
            .ok_or(ProcessInvariantError)?;
        let level = u16::try_from(next.clamp(0, MAX_LEVEL)).map_err(|_| ProcessInvariantError)?;
        // Publish only after all checked arithmetic succeeds. No remainder/clock exists.
        self.level_permille = level;
        self.applied = actuators;
        Ok(self.snapshot())
    }
    pub fn reset(&mut self) -> TankSnapshot {
        *self = Self::default();
        self.snapshot()
    }
}
