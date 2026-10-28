//! Deterministic world chronology and discrete-event scheduling.
//!
//! Three time bases exist in PLC Lab and must never be confused:
//! - host wall time: pacing only, owned by the desktop host;
//! - world time ([`WorldTime`]): authoritative simulation chronology for the whole lab;
//! - PLC task time ([`crate::SimulationClock`]): the semantic time observed by one PLC task.
//!
//! The scheduler orders events by `(time, phase, participant, sequence)`. All four
//! components are integers, so ordering is total and identical on every host.

use std::collections::BTreeMap;

/// Microseconds since the start of the current world. Never derived from host time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WorldTime(u64);

impl WorldTime {
    pub const ZERO: Self = Self(0);
    pub fn from_micros(value: u64) -> Self {
        Self(value)
    }
    pub fn as_micros(self) -> u64 {
        self.0
    }
    pub fn checked_add_micros(self, delta: u64) -> Option<Self> {
        self.0.checked_add(delta).map(Self)
    }
    pub fn saturating_sub(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

/// Tie-break class for events scheduled at the same world instant.
///
/// Documented order: external stimuli and scenario actions first, then fault
/// activation, then network deliveries, then device-internal transitions. PLC
/// input sampling and scans are driven by the engine after all events at an
/// instant have been applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    Stimulus = 0,
    Fault = 1,
    Network = 2,
    Device = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventKey {
    pub time: WorldTime,
    pub phase: Phase,
    pub participant: u32,
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleError {
    /// The requested instant precedes the scheduler's current time.
    InPast,
    /// The 64-bit sequence space is exhausted. Unreachable in practice; reported, never wrapped.
    SequenceExhausted,
}
impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InPast => "An event cannot be scheduled before the current world time.",
            Self::SequenceExhausted => "The world event sequence range is exhausted.",
        })
    }
}
impl std::error::Error for ScheduleError {}

/// Deterministic priority queue of world events.
#[derive(Debug, Clone)]
pub struct Scheduler<E> {
    now: WorldTime,
    next_sequence: u64,
    queue: BTreeMap<EventKey, E>,
}

impl<E> Default for Scheduler<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Scheduler<E> {
    pub fn new() -> Self {
        Self {
            now: WorldTime::ZERO,
            next_sequence: 0,
            queue: BTreeMap::new(),
        }
    }
    pub fn now(&self) -> WorldTime {
        self.now
    }
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
    pub fn schedule(
        &mut self,
        time: WorldTime,
        phase: Phase,
        participant: u32,
        event: E,
    ) -> Result<EventKey, ScheduleError> {
        if time < self.now {
            return Err(ScheduleError::InPast);
        }
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .ok_or(ScheduleError::SequenceExhausted)?;
        let key = EventKey {
            time,
            phase,
            participant,
            sequence,
        };
        self.queue.insert(key, event);
        Ok(key)
    }
    /// Remove every event whose payload matches the predicate. Returns the number removed.
    pub fn cancel_where(&mut self, mut predicate: impl FnMut(&E) -> bool) -> usize {
        let keys: Vec<EventKey> = self
            .queue
            .iter()
            .filter(|(_, e)| predicate(e))
            .map(|(k, _)| *k)
            .collect();
        for key in &keys {
            self.queue.remove(key);
        }
        keys.len()
    }
    pub fn peek_key(&self) -> Option<EventKey> {
        self.queue.keys().next().copied()
    }
    /// Pop the earliest event if it is at or before `limit`, advancing `now` to its time.
    pub fn pop_until(&mut self, limit: WorldTime) -> Option<(EventKey, E)> {
        let key = *self.queue.keys().next()?;
        if key.time > limit {
            return None;
        }
        let event = self.queue.remove(&key)?;
        self.now = key.time;
        Some((key, event))
    }
    /// Advance time with no event. Fails if pending events would be skipped.
    pub fn advance_to(&mut self, time: WorldTime) -> Result<(), ScheduleError> {
        if time < self.now {
            return Err(ScheduleError::InPast);
        }
        if self.peek_key().is_some_and(|k| k.time < time) {
            return Err(ScheduleError::InPast);
        }
        self.now = time;
        Ok(())
    }
    pub fn pending(&self) -> impl Iterator<Item = (&EventKey, &E)> {
        self.queue.iter()
    }
}

/// Deterministic 64-bit mixing for seeded, order-independent pseudo-randomness
/// (for example sensor noise). The same `(seed, stream, index)` always yields the
/// same value, independent of how many other values were requested.
pub fn deterministic_hash(seed: u64, stream: u64, index: u64) -> u64 {
    // SplitMix64 finalizer applied to a combined key.
    let mut z = seed
        ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ index.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_time_then_phase_then_participant_then_insertion() {
        let mut s = Scheduler::new();
        let t = WorldTime::from_micros(10);
        s.schedule(t, Phase::Device, 1, "device").unwrap();
        s.schedule(t, Phase::Stimulus, 2, "stimulus-b").unwrap();
        s.schedule(t, Phase::Stimulus, 1, "stimulus-a").unwrap();
        s.schedule(WorldTime::from_micros(5), Phase::Device, 9, "earlier")
            .unwrap();
        s.schedule(t, Phase::Network, 0, "network").unwrap();
        s.schedule(t, Phase::Stimulus, 1, "stimulus-a2").unwrap();
        let mut order = Vec::new();
        while let Some((_, e)) = s.pop_until(WorldTime::from_micros(100)) {
            order.push(e);
        }
        assert_eq!(
            order,
            [
                "earlier",
                "stimulus-a",
                "stimulus-a2",
                "stimulus-b",
                "network",
                "device"
            ]
        );
        assert_eq!(s.now(), t);
    }

    #[test]
    fn past_scheduling_and_skipping_events_are_rejected() {
        let mut s = Scheduler::new();
        s.schedule(WorldTime::from_micros(50), Phase::Device, 0, 1)
            .unwrap();
        assert_eq!(
            s.advance_to(WorldTime::from_micros(60)),
            Err(ScheduleError::InPast)
        );
        assert!(s.pop_until(WorldTime::from_micros(49)).is_none());
        assert_eq!(s.pop_until(WorldTime::from_micros(50)).unwrap().1, 1);
        assert_eq!(
            s.schedule(WorldTime::from_micros(10), Phase::Device, 0, 2),
            Err(ScheduleError::InPast)
        );
        s.advance_to(WorldTime::from_micros(60)).unwrap();
        assert_eq!(s.now().as_micros(), 60);
    }

    #[test]
    fn cancellation_is_exact_and_identical_runs_are_identical() {
        let run = || {
            let mut s = Scheduler::new();
            let mut log = Vec::new();
            for i in 0..20u32 {
                s.schedule(
                    WorldTime::from_micros(u64::from(i % 5) * 10),
                    if i % 2 == 0 {
                        Phase::Device
                    } else {
                        Phase::Network
                    },
                    i % 3,
                    i,
                )
                .unwrap();
            }
            s.cancel_where(|e| *e == 7);
            while let Some((k, e)) = s.pop_until(WorldTime::from_micros(1000)) {
                log.push((k.time.as_micros(), e));
            }
            log
        };
        let a = run();
        assert_eq!(a, run());
        assert!(!a.iter().any(|(_, e)| *e == 7));
        assert_eq!(a.len(), 19);
    }

    #[test]
    fn deterministic_hash_is_stable_and_index_independent() {
        assert_eq!(deterministic_hash(1, 2, 3), deterministic_hash(1, 2, 3));
        assert_ne!(deterministic_hash(1, 2, 3), deterministic_hash(1, 2, 4));
        assert_ne!(deterministic_hash(1, 2, 3), deterministic_hash(2, 2, 3));
    }
}
