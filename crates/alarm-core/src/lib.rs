//! Generic industrial alarm model (ADR-0037): one lifecycle per alarm, operator
//! acknowledgment kept apart from the condition, an on-delay measured in world time and a
//! bounded, deterministically ordered transition history.
//!
//! Lifecycle (vendor neutral):
//!
//! | State                     | condition holds        | condition clears              | acknowledge        |
//! | ------------------------- | ---------------------- | ----------------------------- | ------------------ |
//! | `Normal`                  | -> `ActiveUnacknowledged` (after the on-delay) | -                | refused            |
//! | `ActiveUnacknowledged`    | -                      | -> `ReturnedUnacknowledged`   | -> `ActiveAcknowledged` |
//! | `ActiveAcknowledged`      | -                      | -> `Normal`                   | refused            |
//! | `ReturnedUnacknowledged`  | -> `ActiveUnacknowledged` (after the on-delay) | -                | -> `Normal`        |
//!
//! Acknowledging never changes the condition: an acknowledged alarm stays active until the
//! condition itself clears. Only transitions are recorded, so a condition that holds for
//! many evaluations produces one activation.

use simulation_core::WorldTime;
use std::collections::VecDeque;

/// History entries kept by default (the newest are kept).
pub const HISTORY_CAPACITY: usize = 500;

/// Generic priority, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlarmPriority {
    Information,
    Warning,
    Alarm,
    Critical,
}

/// What kind of plant object raises the alarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlarmCategory {
    Process,
    Hardware,
    Network,
}

/// Lifecycle state. `Normal` is inactive with nothing left to acknowledge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlarmState {
    Normal,
    ActiveUnacknowledged,
    ActiveAcknowledged,
    ReturnedUnacknowledged,
}
impl AlarmState {
    pub fn is_active(self) -> bool {
        matches!(self, Self::ActiveUnacknowledged | Self::ActiveAcknowledged)
    }
    pub fn needs_acknowledgment(self) -> bool {
        matches!(
            self,
            Self::ActiveUnacknowledged | Self::ReturnedUnacknowledged
        )
    }
    fn acknowledged(self) -> Option<Self> {
        match self {
            Self::ActiveUnacknowledged => Some(Self::ActiveAcknowledged),
            Self::ReturnedUnacknowledged => Some(Self::Normal),
            Self::Normal | Self::ActiveAcknowledged => None,
        }
    }
}

/// One configured alarm. The condition itself is evaluated by the owner of the plant
/// model; the engine only sees whether it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlarmDefinition {
    /// Canonical identity, stable across sessions (for example `tank.high-level`).
    pub id: String,
    /// Generic condition code (for example `HIGH_LEVEL`).
    pub code: String,
    /// Canonical identity of the plant object the condition belongs to.
    pub source: String,
    /// Display name of the source.
    pub source_label: String,
    pub category: AlarmCategory,
    pub priority: AlarmPriority,
    /// Canonical English message; views may present it localized by `code`.
    pub message: String,
    /// The condition must hold this long before the alarm activates (0: at once).
    pub on_delay_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlarmTransition {
    /// The condition held (past the on-delay).
    Activated,
    /// The operator acknowledged the alarm.
    Acknowledged,
    /// The condition cleared.
    ReturnedToNormal,
    /// The alarm was removed from the configuration while not normal.
    Withdrawn,
}

/// One recorded state change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlarmEvent {
    /// Position among all transitions of this engine, strictly increasing.
    pub seq: u64,
    pub time: WorldTime,
    pub definition: AlarmDefinition,
    pub transition: AlarmTransition,
    pub from: AlarmState,
    pub to: AlarmState,
}

/// The current state of one alarm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlarmStatus {
    pub definition: AlarmDefinition,
    pub state: AlarmState,
    /// Whether the condition held at the latest evaluation (also during the on-delay).
    pub condition: bool,
    /// When the current or latest activation began.
    pub activated: Option<WorldTime>,
    /// Time of the latest transition.
    pub changed: Option<WorldTime>,
    /// Acknowledgments given since the engine was created.
    pub acknowledgments: u32,
    pending: Option<WorldTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlarmError {
    DuplicateId(String),
    Unknown(String),
    NothingToAcknowledge(String),
}
impl std::fmt::Display for AlarmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "alarm {id} is configured twice"),
            Self::Unknown(id) => write!(f, "there is no alarm {id}"),
            Self::NothingToAcknowledge(id) => {
                write!(f, "alarm {id} has nothing to acknowledge")
            }
        }
    }
}
impl std::error::Error for AlarmError {}

struct History {
    events: VecDeque<AlarmEvent>,
    capacity: usize,
    next_seq: u64,
}
impl History {
    fn record(
        &mut self,
        status: &AlarmStatus,
        time: WorldTime,
        transition: AlarmTransition,
        from: AlarmState,
    ) -> AlarmEvent {
        let event = AlarmEvent {
            seq: self.next_seq,
            time,
            definition: status.definition.clone(),
            transition,
            from,
            to: status.state,
        };
        self.next_seq += 1;
        if self.events.len() == self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event.clone());
        event
    }
}

/// The alarms of one plant and their history. Alarms are kept and evaluated in the order
/// of their ids, so transitions at the same instant are always recorded in the same order.
pub struct AlarmEngine {
    alarms: Vec<AlarmStatus>,
    history: History,
}

fn sorted(mut definitions: Vec<AlarmDefinition>) -> Result<Vec<AlarmDefinition>, AlarmError> {
    definitions.sort_by(|a, b| a.id.cmp(&b.id));
    if let Some(pair) = definitions.windows(2).find(|p| p[0].id == p[1].id) {
        return Err(AlarmError::DuplicateId(pair[0].id.clone()));
    }
    Ok(definitions)
}

fn normal(definition: AlarmDefinition) -> AlarmStatus {
    AlarmStatus {
        definition,
        state: AlarmState::Normal,
        condition: false,
        activated: None,
        changed: None,
        acknowledgments: 0,
        pending: None,
    }
}

impl AlarmEngine {
    /// An engine with every alarm normal; `capacity` bounds the history (at least 1).
    pub fn new(definitions: Vec<AlarmDefinition>, capacity: usize) -> Result<Self, AlarmError> {
        Ok(Self {
            alarms: sorted(definitions)?.into_iter().map(normal).collect(),
            history: History {
                events: VecDeque::new(),
                capacity: capacity.max(1),
                next_seq: 0,
            },
        })
    }

    /// Replace the configuration. Alarms whose id remains keep their state; an alarm that
    /// is removed while not normal is withdrawn with a recorded event.
    pub fn configure(
        &mut self,
        definitions: Vec<AlarmDefinition>,
        now: WorldTime,
    ) -> Result<Vec<AlarmEvent>, AlarmError> {
        let definitions = sorted(definitions)?;
        let mut events = Vec::new();
        let mut previous = std::mem::take(&mut self.alarms);
        for status in previous.iter_mut() {
            if status.state != AlarmState::Normal
                && !definitions.iter().any(|d| d.id == status.definition.id)
            {
                let from = status.state;
                status.state = AlarmState::Normal;
                let event = self
                    .history
                    .record(status, now, AlarmTransition::Withdrawn, from);
                events.push(event);
            }
        }
        self.alarms = definitions
            .into_iter()
            .map(|definition| {
                match previous
                    .iter()
                    .position(|s| s.definition.id == definition.id)
                {
                    Some(i) => AlarmStatus {
                        definition,
                        ..previous.swap_remove(i)
                    },
                    None => normal(definition),
                }
            })
            .collect();
        Ok(events)
    }

    /// Evaluate every condition at `now` and record the resulting transitions, in id order.
    pub fn update(
        &mut self,
        now: WorldTime,
        mut condition: impl FnMut(&AlarmDefinition) -> bool,
    ) -> Vec<AlarmEvent> {
        let mut events = Vec::new();
        for status in &mut self.alarms {
            let holds = condition(&status.definition);
            status.condition = holds;
            let from = status.state;
            if holds {
                let since = *status.pending.get_or_insert(now);
                if !from.is_active() && now.saturating_sub(since) >= status.definition.on_delay_us {
                    status.state = AlarmState::ActiveUnacknowledged;
                    status.activated = Some(now);
                    status.changed = Some(now);
                    let transition = AlarmTransition::Activated;
                    events.push(self.history.record(status, now, transition, from));
                }
            } else {
                status.pending = None;
                let to = match from {
                    AlarmState::ActiveUnacknowledged => AlarmState::ReturnedUnacknowledged,
                    AlarmState::ActiveAcknowledged => AlarmState::Normal,
                    AlarmState::Normal | AlarmState::ReturnedUnacknowledged => continue,
                };
                status.state = to;
                status.changed = Some(now);
                let transition = AlarmTransition::ReturnedToNormal;
                events.push(self.history.record(status, now, transition, from));
            }
        }
        events
    }

    /// The operator acknowledges one alarm. The condition is not touched.
    pub fn acknowledge(&mut self, id: &str, now: WorldTime) -> Result<AlarmEvent, AlarmError> {
        let status = self
            .alarms
            .iter_mut()
            .find(|s| s.definition.id == id)
            .ok_or_else(|| AlarmError::Unknown(id.to_string()))?;
        let from = status.state;
        let to = from
            .acknowledged()
            .ok_or_else(|| AlarmError::NothingToAcknowledge(id.to_string()))?;
        status.state = to;
        status.changed = Some(now);
        status.acknowledgments += 1;
        let transition = AlarmTransition::Acknowledged;
        Ok(self.history.record(status, now, transition, from))
    }

    /// Acknowledge the listed alarms, in id order. Either every listed alarm waits for an
    /// acknowledgment and all are acknowledged, or nothing changes.
    pub fn acknowledge_many(
        &mut self,
        ids: &[&str],
        now: WorldTime,
    ) -> Result<Vec<AlarmEvent>, AlarmError> {
        let mut ids = ids.to_vec();
        ids.sort_unstable();
        ids.dedup();
        for id in &ids {
            let status = self
                .get(id)
                .ok_or_else(|| AlarmError::Unknown(id.to_string()))?;
            if !status.state.needs_acknowledgment() {
                return Err(AlarmError::NothingToAcknowledge(id.to_string()));
            }
        }
        ids.iter().map(|id| self.acknowledge(id, now)).collect()
    }

    /// Acknowledge every alarm that waits for it, in id order.
    pub fn acknowledge_all(&mut self, now: WorldTime) -> Vec<AlarmEvent> {
        let waiting: Vec<String> = self
            .alarms
            .iter()
            .filter(|s| s.state.needs_acknowledgment())
            .map(|s| s.definition.id.clone())
            .collect();
        waiting
            .iter()
            .filter_map(|id| self.acknowledge(id, now).ok())
            .collect()
    }

    /// Every configured alarm, in id order.
    pub fn alarms(&self) -> &[AlarmStatus] {
        &self.alarms
    }
    pub fn get(&self, id: &str) -> Option<&AlarmStatus> {
        self.alarms.iter().find(|s| s.definition.id == id)
    }
    /// The kept history, oldest first.
    pub fn history(&self) -> impl DoubleEndedIterator<Item = &AlarmEvent> + ExactSizeIterator {
        self.history.events.iter()
    }
    pub fn capacity(&self) -> usize {
        self.history.capacity
    }
    /// Transitions recorded since the engine was created, including those no longer kept.
    pub fn recorded(&self) -> u64 {
        self.history.next_seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AlarmState::*;
    use AlarmTransition::*;

    fn definition(id: &str, on_delay_us: u64) -> AlarmDefinition {
        AlarmDefinition {
            id: id.into(),
            code: "HIGH_LEVEL".into(),
            source: id.into(),
            source_label: id.into(),
            category: AlarmCategory::Process,
            priority: AlarmPriority::Alarm,
            message: format!("{id} is high"),
            on_delay_us,
        }
    }
    fn at(ms: u64) -> WorldTime {
        WorldTime::from_micros(ms * 1000)
    }
    fn engine(ids: &[&str]) -> AlarmEngine {
        let definitions = ids.iter().map(|id| definition(id, 0)).collect();
        AlarmEngine::new(definitions, HISTORY_CAPACITY).unwrap()
    }
    fn state(engine: &AlarmEngine, id: &str) -> AlarmState {
        engine.get(id).unwrap().state
    }
    fn transitions(engine: &AlarmEngine) -> Vec<(AlarmTransition, AlarmState)> {
        engine.history().map(|e| (e.transition, e.to)).collect()
    }

    #[test]
    fn acknowledging_an_active_alarm_keeps_it_active_until_the_condition_clears() {
        let mut alarms = engine(&["tank.high-level"]);
        alarms.update(at(0), |_| true);
        assert_eq!(state(&alarms, "tank.high-level"), ActiveUnacknowledged);
        alarms.acknowledge("tank.high-level", at(100)).unwrap();
        // The condition still holds: acknowledged, but still active.
        alarms.update(at(200), |_| true);
        let status = alarms.get("tank.high-level").unwrap();
        assert_eq!((status.state, status.condition), (ActiveAcknowledged, true));
        alarms.update(at(300), |_| false);
        assert_eq!(state(&alarms, "tank.high-level"), Normal);
        assert_eq!(
            transitions(&alarms),
            [
                (Activated, ActiveUnacknowledged),
                (Acknowledged, ActiveAcknowledged),
                (ReturnedToNormal, Normal),
            ]
        );
    }

    #[test]
    fn a_returned_alarm_waits_for_acknowledgment_and_can_activate_again() {
        let mut alarms = engine(&["tank.high-level"]);
        alarms.update(at(0), |_| true);
        alarms.update(at(100), |_| false);
        assert_eq!(state(&alarms, "tank.high-level"), ReturnedUnacknowledged);
        alarms.update(at(200), |_| true);
        assert_eq!(state(&alarms, "tank.high-level"), ActiveUnacknowledged);
        alarms.update(at(300), |_| false);
        alarms.acknowledge("tank.high-level", at(400)).unwrap();
        assert_eq!(state(&alarms, "tank.high-level"), Normal);
        let status = alarms.get("tank.high-level").unwrap();
        assert_eq!(
            (status.activated, status.acknowledgments),
            (Some(at(200)), 1)
        );
    }

    #[test]
    fn a_holding_condition_is_recorded_once() {
        let mut alarms = engine(&["tank.high-level"]);
        for ms in 0..1000 {
            alarms.update(at(ms), |_| true);
        }
        assert_eq!(alarms.recorded(), 1);
        assert_eq!(alarms.history().len(), 1);
    }

    #[test]
    fn the_on_delay_filters_short_conditions() {
        let definitions = vec![definition("valve.process-timeout", 3_000_000)];
        let mut alarms = AlarmEngine::new(definitions, HISTORY_CAPACITY).unwrap();
        alarms.update(at(0), |_| true);
        alarms.update(at(2_000), |_| true);
        alarms.update(at(2_500), |_| false);
        alarms.update(at(3_000), |_| true);
        alarms.update(at(5_900), |_| true);
        assert_eq!(alarms.recorded(), 0, "never held for 3 s");
        let status = alarms.get("valve.process-timeout").unwrap();
        assert_eq!((status.state, status.condition), (Normal, true));
        alarms.update(at(6_000), |_| true);
        assert_eq!(
            state(&alarms, "valve.process-timeout"),
            ActiveUnacknowledged
        );
        assert_eq!(
            alarms.get("valve.process-timeout").unwrap().activated,
            Some(at(6_000))
        );
    }

    #[test]
    fn only_alarms_waiting_for_it_can_be_acknowledged() {
        let mut alarms = engine(&["a", "b"]);
        let unknown = alarms.acknowledge("c", at(0));
        assert_eq!(unknown, Err(AlarmError::Unknown("c".into())));
        let nothing = alarms.acknowledge("a", at(0));
        assert_eq!(nothing, Err(AlarmError::NothingToAcknowledge("a".into())));
        alarms.update(at(0), |_| true);
        alarms.acknowledge("a", at(1)).unwrap();
        let again = alarms.acknowledge("a", at(2));
        assert_eq!(again, Err(AlarmError::NothingToAcknowledge("a".into())));
        let all = alarms.acknowledge_all(at(3));
        let ids: Vec<&str> = all.iter().map(|e| e.definition.id.as_str()).collect();
        assert_eq!(ids, ["b"]);
        assert!(alarms.acknowledge_all(at(4)).is_empty());
    }

    #[test]
    fn acknowledging_a_selection_is_all_or_nothing() {
        let mut alarms = engine(&["a", "b", "c"]);
        alarms.update(at(0), |d| d.id != "c");
        let refused = alarms.acknowledge_many(&["b", "c"], at(1));
        assert_eq!(refused, Err(AlarmError::NothingToAcknowledge("c".into())));
        assert_eq!(state(&alarms, "b"), ActiveUnacknowledged, "nothing changed");
        let unknown = alarms.acknowledge_many(&["a", "x"], at(1));
        assert_eq!(unknown, Err(AlarmError::Unknown("x".into())));
        let done = alarms.acknowledge_many(&["b", "a", "b"], at(2)).unwrap();
        let ids: Vec<&str> = done.iter().map(|e| e.definition.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(state(&alarms, "a"), ActiveAcknowledged);
    }

    #[test]
    fn transitions_at_the_same_instant_follow_the_alarm_ids() {
        let run = |ids: &[&str]| {
            let mut alarms = engine(ids);
            alarms.update(at(0), |_| true);
            alarms.acknowledge_all(at(0));
            alarms.update(at(0), |_| false);
            alarms
                .history()
                .map(|e| (e.seq, e.definition.id.clone(), e.transition))
                .collect::<Vec<_>>()
        };
        let forward = run(&["pump.dry-run", "tank.high-level", "tank.low-level"]);
        let backward = run(&["tank.low-level", "tank.high-level", "pump.dry-run"]);
        assert_eq!(forward, backward);
        let seqs: Vec<u64> = forward.iter().map(|e| e.0).collect();
        assert_eq!(seqs, (0..9).collect::<Vec<_>>());
        assert_eq!(forward[0].1, "pump.dry-run");
    }

    #[test]
    fn the_history_keeps_the_newest_entries_within_its_capacity() {
        let mut alarms = AlarmEngine::new(vec![definition("a", 0)], 4).unwrap();
        for ms in 0..10 {
            alarms.update(at(ms), |_| ms % 2 == 0);
            alarms.acknowledge_all(at(ms));
        }
        assert_eq!(alarms.history().len(), 4);
        assert_eq!(alarms.recorded(), 15);
        let seqs: Vec<u64> = alarms.history().map(|e| e.seq).collect();
        assert_eq!(seqs, [11, 12, 13, 14]);
    }

    #[test]
    fn a_new_configuration_keeps_known_alarms_and_withdraws_removed_ones() {
        let mut alarms = engine(&["a", "b"]);
        alarms.update(at(0), |_| true);
        let events = alarms
            .configure(vec![definition("a", 0), definition("c", 0)], at(10))
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!((events[0].transition, events[0].to), (Withdrawn, Normal));
        assert_eq!(events[0].definition.id, "b");
        assert_eq!(state(&alarms, "a"), ActiveUnacknowledged);
        assert_eq!(state(&alarms, "c"), Normal);
        assert!(alarms.get("b").is_none());
        let twice = vec![definition("a", 0), definition("a", 0)];
        assert_eq!(
            alarms.configure(twice, at(20)),
            Err(AlarmError::DuplicateId("a".into()))
        );
    }
}
