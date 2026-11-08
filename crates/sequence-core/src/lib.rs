//! Sequence monitor (ADR-0043): observes a PLC program that implements a documented step
//! sequence and reports what it does against the definition.
//!
//! The PLC program is the only controller. The monitor reads one snapshot per cycle (the
//! same [`CycleFacts`] grading reads), never writes PLC memory or the plant, and derives
//! the sequence state, step timers, permissive and interlock evaluations, events and
//! faults deterministically from world time.

use grading_core::{CycleFacts, CycleOutcome, EvidenceValue, compare, condition_holds};
use lab_model::{
    Identifier,
    exercise::{Comparison, Condition, EvidenceLayer, Literal},
    sequence::{InterlockReaction, SequenceDefinition, StepRole},
};
use std::collections::{BTreeSet, VecDeque};

pub use lab_model::sequence;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The newest events the monitor keeps; all events are counted.
pub const HISTORY_CAPACITY: usize = 200;

/// Why a new monitor run began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStart {
    /// The CPU started warm (retained variables kept).
    Warm,
    /// The CPU started cold (all memory initialized).
    Cold,
    /// The monitor observed an executing program for the first time.
    Attached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SequenceState {
    /// No loaded program provides the step variable.
    Unbound,
    /// The CPU is not executing scans.
    Stopped,
    Idle,
    Running,
    Held,
    Completed,
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Inactive,
    Active,
    Held,
    /// Left through a defined transition in this run.
    Completed,
    Faulted,
}

/// A difference between what the program did and the definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deviation {
    UnknownStep,
    UndefinedTransition,
    ConditionNotMet,
    PermissiveViolated,
    InterlockViolated,
    ProgressDuringHold,
    ProgressAfterFault,
}
impl Deviation {
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownStep => "unknownStep",
            Self::UndefinedTransition => "undefinedTransition",
            Self::ConditionNotMet => "conditionNotMet",
            Self::PermissiveViolated => "permissiveViolated",
            Self::InterlockViolated => "interlockViolated",
            Self::ProgressDuringHold => "progressDuringHold",
            Self::ProgressAfterFault => "progressAfterFault",
        }
    }
}

/// Why the sequence is faulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultReason {
    StepTimeout,
    Deviation(Deviation),
}
impl FaultReason {
    pub fn code(self) -> &'static str {
        match self {
            Self::StepTimeout => "stepTimeout",
            Self::Deviation(d) => d.code(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceEventKind {
    RunStarted {
        start: RunStart,
    },
    TransitionCompleted,
    SequenceStarted,
    SequenceCompleted,
    SequenceAborted,
    /// A start from the initial step was requested but refused (permissive or interlock).
    StartBlocked,
    /// A later transition's condition held but a permissive did not.
    TransitionBlocked,
    InterlockActivated,
    InterlockCleared,
    Held,
    Resumed,
    StepTimeout,
    Deviation(Deviation),
    FaultCleared,
}
impl SequenceEventKind {
    /// Stable reason code.
    pub fn code(self) -> &'static str {
        match self {
            Self::RunStarted { .. } => "runStarted",
            Self::TransitionCompleted => "transitionCompleted",
            Self::SequenceStarted => "sequenceStarted",
            Self::SequenceCompleted => "sequenceCompleted",
            Self::SequenceAborted => "sequenceAborted",
            Self::StartBlocked => "startBlocked",
            Self::TransitionBlocked => "transitionBlocked",
            Self::InterlockActivated => "interlockActivated",
            Self::InterlockCleared => "interlockCleared",
            Self::Held => "held",
            Self::Resumed => "resumed",
            Self::StepTimeout => "stepTimeout",
            Self::Deviation(d) => d.code(),
            Self::FaultCleared => "faultCleared",
        }
    }
    pub fn is_fault(self) -> bool {
        matches!(self, Self::StepTimeout | Self::Deviation(_))
    }
}

/// An evidence value an event's decision was based on.
#[derive(Debug, Clone, PartialEq)]
pub struct Observed {
    pub signal: Identifier,
    pub layer: EvidenceLayer,
    pub value: Option<EvidenceValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceEvent {
    /// Strictly increasing within a session.
    pub seq: u64,
    pub time_us: u64,
    pub cycle: u64,
    pub sequence: Identifier,
    pub kind: SequenceEventKind,
    /// The step concerned: the step reached, held, timed out or blocked.
    pub step: Option<Identifier>,
    /// The step left by a step change.
    pub from: Option<Identifier>,
    pub transition: Option<Identifier>,
    pub interlock: Option<Identifier>,
    /// Unsatisfied permissives (blocked or violated).
    pub permissives: Vec<Identifier>,
    /// Interlocks blocking a start.
    pub interlocks: Vec<Identifier>,
    /// The step variable's value (for an unknown step).
    pub step_value: Option<i16>,
    pub values: Vec<Observed>,
}

/// One comparison of a condition with its observed value.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionRow {
    pub signal: Identifier,
    pub layer: EvidenceLayer,
    pub op: Comparison,
    pub expected: Literal,
    pub actual: Option<EvidenceValue>,
    pub satisfied: Option<bool>,
    /// Inside an odd number of NOTs: the condition needs this comparison to be FALSE.
    pub negated: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConditionStatus {
    /// `None` when evidence is not available.
    pub satisfied: Option<bool>,
    pub rows: Vec<ConditionRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissiveStatus {
    pub id: Identifier,
    pub condition: ConditionStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionStatus {
    pub id: Identifier,
    /// The transition leaves the active step.
    pub from_active: bool,
    pub condition: ConditionStatus,
    pub permissives: Vec<PermissiveStatus>,
    /// Active interlocks that block it.
    pub blocked_by: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterlockStatus {
    pub id: Identifier,
    /// It applies to the active step.
    pub applies: bool,
    /// Its condition holds where it applies.
    pub active: bool,
    pub condition: ConditionStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceStatus {
    pub state: SequenceState,
    pub active: Option<Identifier>,
    pub previous: Option<Identifier>,
    pub step_value: Option<i16>,
    pub elapsed_us: u64,
    pub timeout_us: Option<u64>,
    pub fault: Option<FaultReason>,
    pub fault_step: Option<Identifier>,
    /// Aligned with the definition's steps.
    pub steps: Vec<StepState>,
    pub transitions: Vec<TransitionStatus>,
    pub interlocks: Vec<InterlockStatus>,
    pub time_us: u64,
    pub cycle: u64,
}

#[derive(Debug, Clone, Default)]
struct Run {
    attached: bool,
    bound: bool,
    scanning: bool,
    active: Option<usize>,
    previous: Option<usize>,
    step_value: Option<i16>,
    elapsed_us: u64,
    timed_out: bool,
    fault: Option<(FaultReason, Option<usize>)>,
    held: bool,
    completed: BTreeSet<usize>,
    interlock_active: Vec<bool>,
    interlock_since: Vec<Option<u64>>,
    interlock_reported: Vec<bool>,
    blocked: Vec<bool>,
    unknown: Option<i16>,
}

pub struct SequenceMonitor {
    def: SequenceDefinition,
    initial: usize,
    run: Run,
    status: SequenceStatus,
    history: VecDeque<SequenceEvent>,
    total: u64,
    next_seq: u64,
}

fn status_of(condition: &Condition, facts: &CycleFacts) -> ConditionStatus {
    fn leaves(c: &Condition, facts: &CycleFacts, negated: bool, out: &mut Vec<ConditionRow>) {
        match c {
            Condition::All { of } | Condition::Any { of } => {
                of.iter().for_each(|x| leaves(x, facts, negated, out))
            }
            Condition::Not { of } => leaves(of, facts, !negated, out),
            Condition::Compare {
                signal,
                layer,
                op,
                value,
            } => {
                let actual = facts.layer(*layer).get(signal.as_str()).copied();
                out.push(ConditionRow {
                    signal: signal.clone(),
                    layer: *layer,
                    op: *op,
                    expected: *value,
                    actual,
                    satisfied: actual.map(|a| compare(*op, a, *value)),
                    negated,
                });
            }
        }
    }
    let mut rows = Vec::new();
    leaves(condition, facts, false, &mut rows);
    ConditionStatus {
        satisfied: condition_holds(condition, facts),
        rows,
    }
}

fn observed(status: &ConditionStatus) -> Vec<Observed> {
    let mut out: Vec<Observed> = Vec::new();
    for r in &status.rows {
        if !out
            .iter()
            .any(|o| o.signal == r.signal && o.layer == r.layer)
        {
            out.push(Observed {
                signal: r.signal.clone(),
                layer: r.layer,
                value: r.actual,
            });
        }
    }
    out
}

fn holds(condition: &Condition, facts: &CycleFacts) -> bool {
    condition_holds(condition, facts) == Some(true)
}

fn step_number(value: Option<&EvidenceValue>) -> Option<i16> {
    match value {
        Some(EvidenceValue::Number(n))
            if n.fract() == 0.0 && *n >= f64::from(i16::MIN) && *n <= f64::from(i16::MAX) =>
        {
            Some(*n as i16)
        }
        _ => None,
    }
}

/// A new event, filled by the caller.
struct Draft {
    kind: SequenceEventKind,
    step: Option<usize>,
    from: Option<usize>,
    transition: Option<usize>,
    interlock: Option<usize>,
    permissives: Vec<Identifier>,
    interlocks: Vec<Identifier>,
    step_value: Option<i16>,
    values: Vec<Observed>,
}
impl Draft {
    fn new(kind: SequenceEventKind) -> Self {
        Self {
            kind,
            step: None,
            from: None,
            transition: None,
            interlock: None,
            permissives: Vec::new(),
            interlocks: Vec::new(),
            step_value: None,
            values: Vec::new(),
        }
    }
    fn step(mut self, step: Option<usize>) -> Self {
        self.step = step;
        self
    }
    fn left(mut self, from: Option<usize>) -> Self {
        self.from = from;
        self
    }
    fn transition(mut self, t: usize) -> Self {
        self.transition = Some(t);
        self
    }
    fn interlock(mut self, i: Option<usize>) -> Self {
        self.interlock = i;
        self
    }
    fn values(mut self, values: Vec<Observed>) -> Self {
        self.values = values;
        self
    }
}

impl SequenceMonitor {
    /// A monitor for a validated definition (see `lab_model::validate_exercise`).
    pub fn new(def: SequenceDefinition) -> Result<Self, String> {
        let initial = def
            .steps
            .iter()
            .position(|s| s.role == StepRole::Initial)
            .ok_or_else(|| format!("sequence '{}' has no initial step", def.id))?;
        let status = SequenceStatus {
            state: SequenceState::Unbound,
            active: None,
            previous: None,
            step_value: None,
            elapsed_us: 0,
            timeout_us: None,
            fault: None,
            fault_step: None,
            steps: vec![StepState::Inactive; def.steps.len()],
            transitions: Vec::new(),
            interlocks: Vec::new(),
            time_us: 0,
            cycle: 0,
        };
        let mut monitor = Self {
            run: Run::default(),
            initial,
            status,
            history: VecDeque::new(),
            total: 0,
            next_seq: 1,
            def,
        };
        monitor.reset_run();
        Ok(monitor)
    }

    pub fn definition(&self) -> &SequenceDefinition {
        &self.def
    }
    pub fn status(&self) -> &SequenceStatus {
        &self.status
    }
    /// The newest events, oldest first.
    pub fn history(&self) -> impl DoubleEndedIterator<Item = &SequenceEvent> {
        self.history.iter()
    }
    pub fn total_events(&self) -> u64 {
        self.total
    }
    /// The active step timed out (until the step changes or the run restarts).
    pub fn timed_out(&self) -> bool {
        self.run.timed_out
    }
    /// A deviation fault is latched.
    pub fn deviation_latched(&self) -> bool {
        matches!(self.run.fault, Some((FaultReason::Deviation(_), _)))
    }
    /// The interlock is active where it applies.
    pub fn interlock_active(&self, index: usize) -> bool {
        self.run
            .interlock_active
            .get(index)
            .copied()
            .unwrap_or(false)
    }

    fn reset_run(&mut self) {
        let n_i = self.def.interlocks.len();
        let n_t = self.def.transitions.len();
        let keep = (self.run.bound, self.run.scanning);
        self.run = Run {
            interlock_active: vec![false; n_i],
            interlock_since: vec![None; n_i],
            interlock_reported: vec![false; n_i],
            blocked: vec![false; n_t],
            ..Run::default()
        };
        (self.run.bound, self.run.scanning) = keep;
    }

    fn id(&self, step: Option<usize>) -> Option<Identifier> {
        step.map(|i| self.def.steps[i].id.clone())
    }

    fn emit(&mut self, out: &mut Vec<SequenceEvent>, facts: &CycleFacts, d: Draft) {
        let event = SequenceEvent {
            seq: self.next_seq,
            time_us: facts.end_us,
            cycle: facts.cycle,
            sequence: self.def.id.clone(),
            kind: d.kind,
            step: self.id(d.step),
            from: self.id(d.from),
            transition: d.transition.map(|t| self.def.transitions[t].id.clone()),
            interlock: d.interlock.map(|i| self.def.interlocks[i].id.clone()),
            permissives: d.permissives,
            interlocks: d.interlocks,
            step_value: d.step_value,
            values: d.values,
        };
        self.next_seq += 1;
        self.total += 1;
        if self.history.len() == HISTORY_CAPACITY {
            self.history.pop_front();
        }
        self.history.push_back(event.clone());
        out.push(event);
    }

    /// Record a deviation and latch it as the fault unless one is latched.
    fn deviate(&mut self, out: &mut Vec<SequenceEvent>, facts: &CycleFacts, d: Draft) {
        if let SequenceEventKind::Deviation(kind) = d.kind
            && self.run.fault.is_none()
        {
            self.run.fault = Some((FaultReason::Deviation(kind), d.step));
        }
        self.emit(out, facts, d);
    }

    fn applies(&self, interlock: usize, step: Option<usize>) -> bool {
        step.is_some_and(|s| {
            self.def.interlocks[interlock]
                .steps
                .contains(&self.def.steps[s].id)
        })
    }

    /// Observe one cycle. `start` is given for the first executed scan after a PLC start.
    pub fn observe(&mut self, facts: &CycleFacts, start: Option<RunStart>) -> Vec<SequenceEvent> {
        let mut out = Vec::new();
        if facts.outcome != Some(CycleOutcome::Scanned) {
            self.run.scanning = false;
            self.refresh(facts);
            return out;
        }
        self.run.scanning = true;
        let raw = facts.outputs.get(self.def.step_signal.as_str());
        let value = match raw {
            None | Some(EvidenceValue::Bool(_)) => {
                // A program without the step variable; the next bound scan starts a new run.
                self.run.bound = false;
                self.run.attached = false;
                self.refresh(facts);
                return out;
            }
            Some(_) => step_number(raw),
        };
        self.run.bound = true;
        let new = value.and_then(|v| self.def.steps.iter().position(|s| s.number == v));
        let normal =
            |i: Option<usize>| i.is_some_and(|i| self.def.steps[i].role == StepRole::Normal);
        let held_now =
            normal(new)
                && self.def.held_signal.as_ref().is_some_and(|h| {
                    facts.outputs.get(h.as_str()) == Some(&EvidenceValue::Bool(true))
                });
        let conditions: Vec<bool> = self
            .def
            .interlocks
            .iter()
            .map(|i| holds(&i.condition, facts))
            .collect();
        let start = start.or((!self.run.attached).then_some(RunStart::Attached));
        let mut changed = false;
        if let Some(kind) = start {
            self.reset_run();
            self.run.attached = true;
            self.run.active = new;
            self.run.step_value = value;
            let mut d = Draft::new(SequenceEventKind::RunStarted { start: kind }).step(new);
            d.step_value = value;
            self.emit(&mut out, facts, d);
            if new.is_none() {
                self.unknown_step(&mut out, facts, value);
            }
        } else if new != self.run.active || (new.is_none() && value != self.run.unknown) {
            changed = true;
            self.step_change(&mut out, facts, new, value, &conditions);
        } else if !held_now && new.is_some() {
            self.run.elapsed_us += facts.end_us.saturating_sub(facts.start_us);
        }
        self.run.step_value = value;
        // HOLD and RESUME of the active step.
        if held_now && !self.run.held {
            self.emit(
                &mut out,
                facts,
                Draft::new(SequenceEventKind::Held).step(new),
            );
        } else if !held_now && self.run.held && !changed {
            self.emit(
                &mut out,
                facts,
                Draft::new(SequenceEventKind::Resumed).step(new),
            );
            for (i, &condition) in conditions.iter().enumerate() {
                let reaction = self.def.interlocks[i].reaction;
                if condition
                    && self.applies(i, new)
                    && matches!(
                        reaction,
                        InterlockReaction::HoldProgress | InterlockReaction::AbortSequence
                    )
                {
                    self.deviate(
                        &mut out,
                        facts,
                        Draft::new(SequenceEventKind::Deviation(Deviation::InterlockViolated))
                            .step(new)
                            .interlock(Some(i)),
                    );
                }
            }
        }
        self.run.held = held_now;
        self.interlocks(&mut out, facts, new, &conditions);
        self.blocked(&mut out, facts, new, &conditions);
        // Step timeout (world time of executed, not held cycles in this activation).
        if let Some(i) = new
            && let Some(ms) = self.def.steps[i].timeout_ms
            && !held_now
            && !self.run.timed_out
            && self.run.elapsed_us >= u64::from(ms) * 1_000
        {
            self.run.timed_out = true;
            if self.run.fault.is_none() {
                self.run.fault = Some((FaultReason::StepTimeout, new));
            }
            self.emit(
                &mut out,
                facts,
                Draft::new(SequenceEventKind::StepTimeout).step(new),
            );
        }
        self.refresh(facts);
        out
    }

    fn unknown_step(
        &mut self,
        out: &mut Vec<SequenceEvent>,
        facts: &CycleFacts,
        value: Option<i16>,
    ) {
        self.run.unknown = value;
        let mut d = Draft::new(SequenceEventKind::Deviation(Deviation::UnknownStep));
        d.step_value = value;
        self.deviate(out, facts, d);
    }

    fn step_change(
        &mut self,
        out: &mut Vec<SequenceEvent>,
        facts: &CycleFacts,
        new: Option<usize>,
        value: Option<i16>,
        conditions: &[bool],
    ) {
        let old = self.run.active;
        self.run.previous = old.or(self.run.previous);
        self.run.active = new;
        self.run.elapsed_us = 0;
        self.run.timed_out = false;
        let Some(n) = new else {
            self.unknown_step(out, facts, value);
            return;
        };
        self.run.unknown = None;
        let to_initial = n == self.initial;
        let was_fault = self.run.fault.is_some();
        if to_initial && was_fault {
            self.run.fault = None;
            self.emit(
                out,
                facts,
                Draft::new(SequenceEventKind::FaultCleared).step(new),
            );
        }
        let transition = old.and_then(|o| {
            self.def
                .transitions
                .iter()
                .position(|t| t.from == self.def.steps[o].id && t.to == self.def.steps[n].id)
        });
        if let Some(t) = transition {
            let o = old.expect("a transition has a source");
            let def = &self.def.transitions[t];
            let condition = status_of(&def.condition, facts);
            let unsatisfied: Vec<Identifier> = def
                .permissives
                .iter()
                .filter(|p| !holds(&p.condition, facts))
                .map(|p| p.id.clone())
                .collect();
            let mut values = observed(&condition);
            for p in &def.permissives {
                for v in observed(&status_of(&p.condition, facts)) {
                    if !values.contains(&v) {
                        values.push(v);
                    }
                }
            }
            if o == self.initial {
                self.run.completed.clear();
            }
            self.run.completed.insert(o);
            self.emit(
                out,
                facts,
                Draft::new(SequenceEventKind::TransitionCompleted)
                    .step(new)
                    .left(old)
                    .transition(t)
                    .values(values.clone()),
            );
            if o == self.initial {
                self.emit(
                    out,
                    facts,
                    Draft::new(SequenceEventKind::SequenceStarted)
                        .step(new)
                        .left(old),
                );
            }
            if self.def.steps[n].role == StepRole::Final {
                self.emit(
                    out,
                    facts,
                    Draft::new(SequenceEventKind::SequenceCompleted)
                        .step(new)
                        .left(old),
                );
            }
            if condition.satisfied != Some(true) {
                self.deviate(
                    out,
                    facts,
                    Draft::new(SequenceEventKind::Deviation(Deviation::ConditionNotMet))
                        .step(new)
                        .left(old)
                        .transition(t)
                        .values(observed(&condition)),
                );
            }
            if !unsatisfied.is_empty() {
                let mut d = Draft::new(SequenceEventKind::Deviation(Deviation::PermissiveViolated))
                    .step(new)
                    .left(old)
                    .transition(t)
                    .values(values);
                d.permissives = unsatisfied;
                self.deviate(out, facts, d);
            }
        } else if to_initial {
            let interlock = (0..self.def.interlocks.len()).find(|&i| {
                conditions[i]
                    && self.def.interlocks[i].reaction == InterlockReaction::AbortSequence
                    && self.applies(i, old)
            });
            self.emit(
                out,
                facts,
                Draft::new(SequenceEventKind::SequenceAborted)
                    .step(new)
                    .left(old)
                    .interlock(interlock),
            );
        } else {
            self.deviate(
                out,
                facts,
                Draft::new(SequenceEventKind::Deviation(Deviation::UndefinedTransition))
                    .step(new)
                    .left(old),
            );
        }
        if !to_initial {
            if self.run.held {
                self.deviate(
                    out,
                    facts,
                    Draft::new(SequenceEventKind::Deviation(Deviation::ProgressDuringHold))
                        .step(new)
                        .left(old),
                );
            }
            if was_fault {
                self.deviate(
                    out,
                    facts,
                    Draft::new(SequenceEventKind::Deviation(Deviation::ProgressAfterFault))
                        .step(new)
                        .left(old),
                );
            }
            for (i, &condition) in conditions.iter().enumerate() {
                if condition && self.applies(i, old) {
                    let values = observed(&status_of(&self.def.interlocks[i].condition, facts));
                    self.deviate(
                        out,
                        facts,
                        Draft::new(SequenceEventKind::Deviation(Deviation::InterlockViolated))
                            .step(new)
                            .left(old)
                            .interlock(Some(i))
                            .values(values),
                    );
                }
            }
        }
    }

    fn interlocks(
        &mut self,
        out: &mut Vec<SequenceEvent>,
        facts: &CycleFacts,
        step: Option<usize>,
        conditions: &[bool],
    ) {
        for (i, &condition) in conditions.iter().enumerate() {
            let active = condition && self.applies(i, step);
            if active != self.run.interlock_active[i] {
                self.run.interlock_active[i] = active;
                let kind = if active {
                    SequenceEventKind::InterlockActivated
                } else {
                    SequenceEventKind::InterlockCleared
                };
                let values = observed(&status_of(&self.def.interlocks[i].condition, facts));
                self.emit(
                    out,
                    facts,
                    Draft::new(kind)
                        .step(step)
                        .interlock(Some(i))
                        .values(values),
                );
            }
            if !active {
                self.run.interlock_since[i] = None;
                self.run.interlock_reported[i] = false;
                continue;
            }
            let interlock = &self.def.interlocks[i];
            if interlock.reaction == InterlockReaction::AbortSequence {
                let since = *self.run.interlock_since[i].get_or_insert(facts.end_us);
                let limit = u64::from(interlock.reaction_ms.unwrap_or(0)) * 1_000;
                if facts.end_us - since > limit && !self.run.interlock_reported[i] {
                    self.run.interlock_reported[i] = true;
                    self.deviate(
                        out,
                        facts,
                        Draft::new(SequenceEventKind::Deviation(Deviation::InterlockViolated))
                            .step(step)
                            .interlock(Some(i)),
                    );
                }
            }
        }
    }

    fn blocking(
        &self,
        transition: usize,
        conditions: &[bool],
        step: Option<usize>,
    ) -> Vec<Identifier> {
        if self.def.transitions[transition].from != self.def.steps[self.initial].id {
            return Vec::new();
        }
        (0..self.def.interlocks.len())
            .filter(|&i| {
                conditions[i]
                    && self.def.interlocks[i].reaction == InterlockReaction::BlockStart
                    && self.applies(i, step)
            })
            .map(|i| self.def.interlocks[i].id.clone())
            .collect()
    }

    fn blocked(
        &mut self,
        out: &mut Vec<SequenceEvent>,
        facts: &CycleFacts,
        step: Option<usize>,
        conditions: &[bool],
    ) {
        for t in 0..self.def.transitions.len() {
            let def = &self.def.transitions[t];
            let from_active = step.is_some_and(|s| self.def.steps[s].id == def.from);
            let unsatisfied: Vec<Identifier> = def
                .permissives
                .iter()
                .filter(|p| !holds(&p.condition, facts))
                .map(|p| p.id.clone())
                .collect();
            let interlocks = self.blocking(t, conditions, step);
            let now = from_active
                && holds(&def.condition, facts)
                && (!unsatisfied.is_empty() || !interlocks.is_empty());
            if now && !self.run.blocked[t] {
                let kind = if def.from == self.def.steps[self.initial].id {
                    SequenceEventKind::StartBlocked
                } else {
                    SequenceEventKind::TransitionBlocked
                };
                let mut values = Vec::new();
                for p in &def.permissives {
                    values.extend(observed(&status_of(&p.condition, facts)));
                }
                let mut d = Draft::new(kind).step(step).transition(t).values(values);
                d.permissives = unsatisfied;
                d.interlocks = interlocks;
                self.emit(out, facts, d);
            }
            self.run.blocked[t] = now;
        }
    }

    fn refresh(&mut self, facts: &CycleFacts) {
        let run = &self.run;
        let state = if !run.bound {
            SequenceState::Unbound
        } else if !run.scanning {
            SequenceState::Stopped
        } else if run.fault.is_some() {
            SequenceState::Faulted
        } else {
            match run.active.map(|i| self.def.steps[i].role) {
                Some(StepRole::Final) => SequenceState::Completed,
                Some(StepRole::Initial) | None => SequenceState::Idle,
                Some(StepRole::Normal) if run.held => SequenceState::Held,
                Some(StepRole::Normal) => SequenceState::Running,
            }
        };
        let steps = (0..self.def.steps.len())
            .map(|i| {
                if run.active == Some(i) && run.bound {
                    if run.fault.is_some() {
                        StepState::Faulted
                    } else if run.held {
                        StepState::Held
                    } else {
                        StepState::Active
                    }
                } else if run.completed.contains(&i) {
                    StepState::Completed
                } else {
                    StepState::Inactive
                }
            })
            .collect();
        let conditions: Vec<bool> = self
            .def
            .interlocks
            .iter()
            .map(|i| holds(&i.condition, facts))
            .collect();
        let transitions = (0..self.def.transitions.len())
            .map(|t| {
                let def = &self.def.transitions[t];
                TransitionStatus {
                    id: def.id.clone(),
                    from_active: run.active.is_some_and(|s| self.def.steps[s].id == def.from),
                    condition: status_of(&def.condition, facts),
                    permissives: def
                        .permissives
                        .iter()
                        .map(|p| PermissiveStatus {
                            id: p.id.clone(),
                            condition: status_of(&p.condition, facts),
                        })
                        .collect(),
                    blocked_by: self.blocking(t, &conditions, run.active),
                }
            })
            .collect();
        let interlocks = self
            .def
            .interlocks
            .iter()
            .enumerate()
            .map(|(i, def)| InterlockStatus {
                id: def.id.clone(),
                applies: self.applies(i, run.active),
                active: run.interlock_active.get(i).copied().unwrap_or(false),
                condition: status_of(&def.condition, facts),
            })
            .collect();
        self.status = SequenceStatus {
            state,
            active: self.id(run.active),
            previous: self.id(run.previous),
            step_value: run.step_value,
            elapsed_us: run.elapsed_us,
            timeout_us: run
                .active
                .and_then(|i| self.def.steps[i].timeout_ms)
                .map(|ms| u64::from(ms) * 1_000),
            fault: run.fault.map(|(r, _)| r),
            fault_step: run.fault.and_then(|(_, s)| self.id(s)),
            steps,
            transitions,
            interlocks,
            time_us: facts.end_us,
            cycle: facts.cycle,
        };
    }
}
