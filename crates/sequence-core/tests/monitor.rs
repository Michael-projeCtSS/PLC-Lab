//! The sequence monitor against scripted cycle facts: states, transitions, permissives,
//! interlocks, timeouts, HOLD/RESUME, restarts, ordering, bounds and determinism.
use grading_core::{CycleFacts, CycleOutcome, EvidenceValue};
use lab_model::{Identifier, sequence::SequenceDefinition};
use sequence_core::{
    Deviation, FaultReason, HISTORY_CAPACITY, RunStart, SequenceEvent, SequenceEventKind as K,
    SequenceMonitor, SequenceState as S, StepState,
};
use std::collections::BTreeSet;

const CYCLE_US: u64 = 10_000;

fn definition() -> SequenceDefinition {
    serde_json::from_str(
        r#"{
        "id": "batch", "name": "Batch", "stepSignal": "step", "heldSignal": "held",
        "steps": [
            {"id": "idle", "number": 0, "name": "Idle", "role": "initial"},
            {"id": "fill", "number": 1, "name": "Fill", "timeoutMs": 100},
            {"id": "drain", "number": 2, "name": "Drain", "timeoutMs": 1000},
            {"id": "done", "number": 3, "name": "Done", "role": "final"}
        ],
        "transitions": [
            {"id": "start", "from": "idle", "to": "fill", "name": "Start",
             "condition": {"type": "compare", "signal": "start", "layer": "inputs", "op": "eq", "value": true},
             "permissives": [{"id": "not-full", "name": "Not full",
                "condition": {"type": "compare", "signal": "high", "layer": "inputs", "op": "eq", "value": false}}]},
            {"id": "full", "from": "fill", "to": "drain", "name": "Full",
             "condition": {"type": "compare", "signal": "high", "layer": "inputs", "op": "eq", "value": true}},
            {"id": "empty", "from": "drain", "to": "done", "name": "Empty",
             "condition": {"type": "compare", "signal": "low", "layer": "inputs", "op": "eq", "value": true}},
            {"id": "again", "from": "done", "to": "idle", "name": "Again",
             "condition": {"type": "compare", "signal": "start", "layer": "inputs", "op": "eq", "value": true}}
        ],
        "interlocks": [
            {"id": "door", "name": "Door open", "reaction": "blockStart", "steps": ["idle"],
             "condition": {"type": "compare", "signal": "door", "layer": "inputs", "op": "eq", "value": true}},
            {"id": "dry", "name": "Tank dry", "reaction": "holdProgress", "steps": ["drain"],
             "condition": {"type": "compare", "signal": "dry", "layer": "inputs", "op": "eq", "value": true}},
            {"id": "stop", "name": "Stop", "reaction": "abortSequence", "steps": ["fill", "drain"], "reactionMs": 20,
             "condition": {"type": "compare", "signal": "stop", "layer": "inputs", "op": "eq", "value": true}}
        ]
    }"#,
    )
    .unwrap()
}

/// A scripted plant: the step the program commits and the inputs it read.
struct Script {
    monitor: SequenceMonitor,
    n: u64,
    events: Vec<SequenceEvent>,
}
impl Script {
    fn new() -> Self {
        Self {
            monitor: SequenceMonitor::new(definition()).unwrap(),
            n: 0,
            events: Vec::new(),
        }
    }
    fn facts(&self, step: Option<i16>, held: bool, inputs: &[&str], scanned: bool) -> CycleFacts {
        let mut f = CycleFacts {
            cycle: self.n,
            start_us: self.n * CYCLE_US,
            end_us: (self.n + 1) * CYCLE_US,
            outcome: Some(if scanned {
                CycleOutcome::Scanned
            } else {
                CycleOutcome::NotScanned
            }),
            ..Default::default()
        };
        if let Some(step) = step {
            f.outputs
                .insert("step".into(), EvidenceValue::Number(f64::from(step)));
        }
        f.outputs.insert("held".into(), EvidenceValue::Bool(held));
        for name in ["start", "high", "low", "door", "dry", "stop"] {
            f.inputs
                .insert(name.into(), EvidenceValue::Bool(inputs.contains(&name)));
        }
        f
    }
    /// One executed scan; returns the event codes it produced.
    fn scan(&mut self, step: i16, inputs: &[&str]) -> Vec<&'static str> {
        self.cycle(Some(step), false, inputs, true, None)
    }
    fn held(&mut self, step: i16, inputs: &[&str]) -> Vec<&'static str> {
        self.cycle(Some(step), true, inputs, true, None)
    }
    fn cycle(
        &mut self,
        step: Option<i16>,
        held: bool,
        inputs: &[&str],
        scanned: bool,
        start: Option<RunStart>,
    ) -> Vec<&'static str> {
        let facts = self.facts(step, held, inputs, scanned);
        let events = self.monitor.observe(&facts, start);
        self.n += 1;
        let codes = events.iter().map(|e| e.kind.code()).collect();
        self.events.extend(events);
        codes
    }
    fn state(&self) -> S {
        self.monitor.status().state
    }
    fn last(&self) -> &SequenceEvent {
        self.events.last().unwrap()
    }
}

fn id(s: &str) -> Identifier {
    Identifier::new(s).unwrap()
}

#[test]
fn initial_state_then_the_first_scan_attaches_a_run() {
    let mut s = Script::new();
    assert_eq!(s.state(), S::Unbound);
    assert_eq!(s.scan(0, &[]), ["runStarted"]);
    assert_eq!(
        s.last().kind,
        K::RunStarted {
            start: RunStart::Attached
        }
    );
    assert_eq!(s.state(), S::Idle);
    assert_eq!(s.monitor.status().active, Some(id("idle")));
    assert_eq!(s.monitor.status().steps[0], StepState::Active);
    // Nothing happens while the program stays in the step.
    assert!(s.scan(0, &[]).is_empty());
}

#[test]
fn a_defined_start_and_its_transitions_complete_the_sequence() {
    let mut s = Script::new();
    s.scan(0, &[]);
    assert_eq!(
        s.scan(1, &["start"]),
        ["transitionCompleted", "sequenceStarted"]
    );
    let started = &s.events[s.events.len() - 2];
    assert_eq!(started.transition, Some(id("start")));
    assert_eq!(started.from, Some(id("idle")));
    assert_eq!(started.step, Some(id("fill")));
    assert!(
        started
            .values
            .iter()
            .any(|v| v.signal.as_str() == "start" && v.value == Some(EvidenceValue::Bool(true)))
    );
    assert_eq!(s.state(), S::Running);
    assert_eq!(s.monitor.status().steps[0], StepState::Completed);
    assert_eq!(s.scan(2, &["high"]), ["transitionCompleted"]);
    assert_eq!(
        s.scan(3, &["low"]),
        ["transitionCompleted", "sequenceCompleted"]
    );
    assert_eq!(s.state(), S::Completed);
    // A new batch clears the completed marks when it starts again.
    assert_eq!(s.scan(0, &["start"]), ["transitionCompleted"]);
    s.scan(1, &["start"]);
    let steps = &s.monitor.status().steps;
    assert_eq!(steps[2], StepState::Inactive);
    assert_eq!(steps[0], StepState::Completed);
    assert!(!s.events.iter().any(|e| e.kind.is_fault()));
}

#[test]
fn an_unsatisfied_permissive_blocks_the_start_once_per_request() {
    let mut s = Script::new();
    s.scan(0, &[]);
    assert_eq!(s.scan(0, &["start", "high"]), ["startBlocked"]);
    assert_eq!(s.last().permissives, vec![id("not-full")]);
    // A held request is one refusal, not one per scan.
    for _ in 0..20 {
        assert!(s.scan(0, &["start", "high"]).is_empty());
    }
    assert_eq!(s.state(), S::Idle, "a permissive is never a fault");
    let t = &s.monitor.status().transitions[0];
    assert!(t.from_active);
    assert_eq!(t.condition.satisfied, Some(true));
    assert_eq!(t.permissives[0].condition.satisfied, Some(false));
    s.scan(0, &["high"]);
    assert_eq!(s.scan(0, &["start", "high"]), ["startBlocked"]);
    // Starting anyway is a violation.
    assert_eq!(
        s.scan(1, &["start", "high"]),
        [
            "transitionCompleted",
            "sequenceStarted",
            "permissiveViolated"
        ]
    );
    assert_eq!(s.state(), S::Faulted);
    assert_eq!(
        s.monitor.status().fault,
        Some(FaultReason::Deviation(Deviation::PermissiveViolated))
    );
}

#[test]
fn interlocks_block_the_start_hold_progress_and_demand_an_abort() {
    // blockStart
    let mut s = Script::new();
    s.scan(0, &[]);
    assert_eq!(s.scan(0, &["door"]), ["interlockActivated"]);
    assert_eq!(s.scan(0, &["door", "start"]), ["startBlocked"]);
    assert_eq!(s.last().interlocks, vec![id("door")]);
    assert_eq!(
        s.monitor.status().transitions[0].blocked_by,
        vec![id("door")]
    );
    let codes = s.scan(1, &["door", "start"]);
    assert!(codes.contains(&"interlockViolated"), "{codes:?}");
    assert_eq!(s.state(), S::Faulted);
    // holdProgress
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    s.scan(2, &["high"]);
    assert_eq!(s.scan(2, &["dry"]), ["interlockActivated"]);
    assert_eq!(s.state(), S::Running, "an interlock alone is not a fault");
    let codes = s.scan(3, &["dry", "low"]);
    assert!(codes.contains(&"interlockViolated"), "{codes:?}");
    // abortSequence: the program returns to the initial step in time.
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    assert_eq!(s.scan(1, &["stop"]), ["interlockActivated"]);
    assert_eq!(
        s.scan(0, &["stop"]),
        ["sequenceAborted", "interlockCleared"]
    );
    let aborted = &s.events[s.events.len() - 2];
    assert_eq!(aborted.interlock, Some(id("stop")));
    assert_eq!(s.state(), S::Idle);
    // ... or too late: one violation after the reaction time, not one per scan.
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    s.scan(1, &["stop"]);
    assert!(s.scan(1, &["stop"]).is_empty(), "within 20 ms");
    assert!(s.scan(1, &["stop"]).is_empty(), "exactly 20 ms");
    assert_eq!(s.scan(1, &["stop"]), ["interlockViolated"]);
    // One violation per activation (the fill step's own timeout comes at 100 ms).
    for _ in 0..5 {
        assert!(s.scan(1, &["stop"]).is_empty());
    }
    assert_eq!(s.state(), S::Faulted);
}

#[test]
fn a_step_times_out_exactly_at_its_world_time_limit_once() {
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]); // fill active at the end of this cycle, elapsed 0
    for i in 1..10 {
        assert!(s.scan(1, &[]).is_empty(), "cycle {i}");
        assert_eq!(s.monitor.status().elapsed_us, i * CYCLE_US);
    }
    assert_eq!(s.scan(1, &[]), ["stepTimeout"]);
    assert_eq!(s.monitor.status().elapsed_us, 100_000);
    assert_eq!(s.monitor.status().timeout_us, Some(100_000));
    assert!(s.monitor.timed_out());
    assert_eq!(s.state(), S::Faulted);
    assert_eq!(s.monitor.status().steps[1], StepState::Faulted);
    for _ in 0..50 {
        assert!(s.scan(1, &[]).is_empty(), "one timeout per activation");
    }
    // A faulted sequence does not continue silently.
    let codes = s.scan(2, &["high"]);
    assert!(codes.contains(&"progressAfterFault"), "{codes:?}");
    assert!(
        !s.monitor.timed_out(),
        "the timeout belongs to the step it ended"
    );
    // Back to the initial step clears the fault.
    assert_eq!(s.scan(0, &[]), ["faultCleared", "sequenceAborted"]);
    assert_eq!(s.state(), S::Idle);
}

#[test]
fn hold_pauses_the_step_timer_and_resume_continues_the_same_step() {
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    for _ in 0..5 {
        s.scan(1, &[]);
    }
    assert_eq!(s.held(1, &[]), ["held"]);
    assert_eq!(s.state(), S::Held);
    assert_eq!(s.monitor.status().steps[1], StepState::Held);
    for _ in 0..100 {
        assert!(s.held(1, &[]).is_empty(), "no timeout while held");
    }
    assert_eq!(s.monitor.status().elapsed_us, 5 * CYCLE_US);
    // The resume cycle ends with the step active and not held: it counts.
    assert_eq!(s.scan(1, &[]), ["resumed"]);
    assert_eq!(s.state(), S::Running);
    assert_eq!(s.monitor.status().active, Some(id("fill")));
    assert_eq!(s.monitor.status().elapsed_us, 6 * CYCLE_US);
    for _ in 0..3 {
        assert!(s.scan(1, &[]).is_empty());
    }
    assert_eq!(s.scan(1, &[]), ["stepTimeout"]);
    // Progress while held is a deviation; an abort is always allowed.
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    s.held(1, &[]);
    let codes = s.held(2, &["high"]);
    assert!(codes.contains(&"progressDuringHold"), "{codes:?}");
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    s.held(1, &[]);
    assert_eq!(s.scan(0, &[]), ["sequenceAborted"]);
    // Resuming into an active abort interlock is a deviation.
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    s.held(1, &[]);
    let codes = s.scan(1, &["stop"]);
    assert_eq!(codes[0], "resumed");
    assert!(codes.contains(&"interlockViolated"), "{codes:?}");
}

#[test]
fn undefined_unknown_and_unmet_changes_are_deviations() {
    let mut s = Script::new();
    s.scan(0, &[]);
    assert_eq!(s.scan(2, &[]), ["undefinedTransition"]);
    assert_eq!(s.state(), S::Faulted);
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    assert_eq!(
        s.scan(2, &[]),
        ["transitionCompleted", "conditionNotMet"],
        "the program advanced although the high level was not reached"
    );
    let mut s = Script::new();
    s.scan(0, &[]);
    assert_eq!(s.scan(7, &[]), ["unknownStep"]);
    assert_eq!(s.last().step_value, Some(7));
    assert!(s.scan(7, &[]).is_empty());
    assert_eq!(s.monitor.status().active, None);
    assert_eq!(s.state(), S::Faulted);
    assert_eq!(s.scan(0, &[]), ["faultCleared", "sequenceAborted"]);
}

#[test]
fn stop_pauses_and_every_plc_start_begins_a_new_run() {
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(1, &["start"]);
    s.scan(1, &[]);
    s.cycle(Some(1), false, &[], false, None);
    assert_eq!(s.state(), S::Stopped);
    let elapsed = s.monitor.status().elapsed_us;
    for _ in 0..50 {
        s.cycle(Some(1), false, &[], false, None);
    }
    assert_eq!(
        s.monitor.status().elapsed_us,
        elapsed,
        "STOP pauses the timer"
    );
    // A warm start with a retained step resumes there with a fresh timer.
    assert_eq!(
        s.cycle(Some(1), false, &[], true, Some(RunStart::Warm)),
        ["runStarted"]
    );
    assert_eq!(s.state(), S::Running);
    assert_eq!(s.monitor.status().elapsed_us, 0);
    // A cold start initializes the step variable: a new run in the initial step, no
    // deviation for the jump.
    s.scan(2, &["high"]);
    assert_eq!(
        s.cycle(Some(0), false, &[], true, Some(RunStart::Cold)),
        ["runStarted"]
    );
    assert_eq!(s.state(), S::Idle);
    assert!(!s.events.iter().any(|e| e.kind.is_fault()));
}

#[test]
fn a_program_without_the_step_variable_is_unbound() {
    let mut s = Script::new();
    s.cycle(None, false, &[], true, None);
    assert_eq!(s.state(), S::Unbound);
    assert_eq!(s.scan(0, &[]), ["runStarted"]);
    s.cycle(None, false, &[], true, None);
    assert_eq!(s.state(), S::Unbound);
    assert_eq!(s.scan(1, &[]), ["runStarted"], "a new program is a new run");
}

fn replay() -> Vec<SequenceEvent> {
    let mut s = Script::new();
    s.scan(0, &[]);
    s.scan(0, &["start", "high"]);
    s.scan(1, &["start"]);
    s.held(1, &["dry"]);
    s.scan(1, &["stop"]);
    s.scan(0, &["stop"]);
    s.scan(2, &[]);
    for _ in 0..120 {
        s.scan(2, &[]);
    }
    s.scan(0, &[]);
    s.events
}

#[test]
fn identical_cycles_give_identical_ordered_events() {
    let a = replay();
    assert_eq!(a, replay());
    assert!(a.windows(2).all(|w| w[0].seq < w[1].seq));
    assert!(a.windows(2).all(|w| w[0].time_us <= w[1].time_us));
    assert!(a.iter().all(|e| e.sequence.as_str() == "batch"));
}

#[test]
fn the_history_is_bounded_and_counts_every_event() {
    let mut s = Script::new();
    s.scan(0, &[]);
    for _ in 0..300 {
        s.scan(0, &["door"]);
        s.scan(0, &[]);
    }
    assert_eq!(s.monitor.history().count(), HISTORY_CAPACITY);
    assert_eq!(s.monitor.total_events(), 601);
    let seqs: Vec<u64> = s.monitor.history().map(|e| e.seq).collect();
    assert!(seqs.windows(2).all(|w| w[1] == w[0] + 1));
    assert_eq!(*seqs.last().unwrap(), 601);
}

#[test]
fn invalid_definitions_are_refused_with_reasons() {
    let evidence: Vec<Identifier> = [
        "step", "held", "start", "high", "low", "door", "dry", "stop",
    ]
    .iter()
    .map(|s| id(s))
    .collect();
    let all: BTreeSet<&Identifier> = evidence.iter().collect();
    let tags: BTreeSet<&Identifier> = evidence[..2].iter().collect();
    assert!(lab_model::sequence::sequence_problems(&definition(), &all, &tags).is_empty());
    let broken = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut v = serde_json::to_value(definition()).unwrap();
        edit(&mut v);
        let d: SequenceDefinition = serde_json::from_value(v).unwrap();
        lab_model::sequence::sequence_problems(&d, &all, &tags).join("; ")
    };
    assert!(broken(&|v| v["steps"][1]["role"] = "initial".into()).contains("exactly one initial"));
    assert!(broken(&|v| v["steps"][2]["number"] = 1.into()).contains("used twice"));
    assert!(broken(&|v| v["transitions"][1]["to"] = "nowhere".into()).contains("missing step"));
    assert!(
        broken(&|v| v["transitions"][1]["condition"]["signal"] = "ghost".into())
            .contains("undefined evidence 'ghost'")
    );
    assert!(broken(&|v| v["stepSignal"] = "start".into()).contains("program tag"));
    assert!(
        broken(&|v| v["interlocks"][2]["reactionMs"] = serde_json::Value::Null)
            .contains("reaction time")
    );
    assert!(
        broken(&|v| v["interlocks"][0]["steps"] = serde_json::json!(["fill"]))
            .contains("initial step only")
    );
    assert!(broken(&|v| v["steps"][0]["timeoutMs"] = 500.into()).contains("normal step"));
    assert!(
        serde_json::from_str::<SequenceDefinition>(r#"{"id": "x", "eval": "1+1"}"#).is_err(),
        "unknown fields such as expressions are refused"
    );
}
