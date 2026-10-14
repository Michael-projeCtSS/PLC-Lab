use plc_types::PlcTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockOverflow;
impl std::fmt::Display for ClockOverflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Virtual time exceeds the supported microsecond range.")
    }
}
impl std::error::Error for ClockOverflow {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationClock {
    current: PlcTime,
}
impl Default for SimulationClock {
    fn default() -> Self {
        Self::new()
    }
}
impl SimulationClock {
    pub fn new() -> Self {
        Self::at(PlcTime::ZERO)
    }
    pub fn at(current: PlcTime) -> Self {
        Self { current }
    }
    pub fn now(self) -> PlcTime {
        self.current
    }
    pub fn advance(&mut self, delta: PlcTime) -> Result<(), ClockOverflow> {
        let next = self
            .current
            .as_micros()
            .checked_add(delta.as_micros())
            .ok_or(ClockOverflow)?;
        self.current = PlcTime::from_micros(next);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starts_at_zero_and_advances_exactly_without_wall_clock() {
        let mut clock = SimulationClock::new();
        assert_eq!(clock.now(), PlcTime::ZERO);
        for delta in [0, 1, 999, 5_000_000] {
            clock.advance(PlcTime::from_micros(delta)).unwrap();
        }
        assert_eq!(clock.now().as_micros(), 5_001_000);
    }
    #[test]
    fn overflow_is_controlled_and_leaves_time_unchanged() {
        let mut clock = SimulationClock::at(PlcTime::from_micros(u64::MAX - 1));
        clock.advance(PlcTime::from_micros(1)).unwrap();
        let before = clock;
        assert_eq!(clock.advance(PlcTime::from_micros(1)), Err(ClockOverflow));
        assert_eq!(clock, before);
        clock.advance(PlcTime::ZERO).unwrap();
        assert_eq!(clock, before);
    }
    #[test]
    fn equal_explicit_advances_produce_equal_clocks() {
        let mut a = SimulationClock::new();
        let mut b = SimulationClock::default();
        for delta in [1, 1_000, 7, u64::MAX - 1_008] {
            assert_eq!(
                a.advance(PlcTime::from_micros(delta)),
                b.advance(PlcTime::from_micros(delta))
            );
            assert_eq!(a, b);
        }
        assert_eq!(a.now().as_micros(), u64::MAX);
    }
}
