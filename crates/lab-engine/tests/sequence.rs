//! The sequence monitor in the real engine (ADR-0043): the tank batch sequence observed
//! at the PLC cycle boundary with real process feedback, alarms, restarts, forces, world
//! resets and isolated assessment, and proof that observing never changes the plant.
use lab_engine::{
    AlarmState, AlarmTransition, AssessmentOutcome, LabEventKind, LabSession, OperatorAction,
    SessionConfig, SignalValue, StartKind, assess,
    sequence_core::{
        Deviation, FaultReason, RunStart, SequenceEvent, SequenceEventKind as K, SequenceState as S,
    },
};
use lab_model::{
    exercise::{ExerciseDefinition, PlcModeRequest},
    fault::FaultInjection,
    parse_json,
    profile::PlatformProfile,
    scene::SceneDefinition,
};

const SCENE: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/tank-filling.json");
const EXERCISE: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/ex-tank-sequence.json");
const SOLUTION: &str = include_str!("../../../tests/conformance/solutions/ex.tank-sequence.st");
const TIMEOUT_ALARM: &str = "sequence.tank-batch.step-timeout";

fn exercise() -> ExerciseDefinition {
    parse_json(EXERCISE).unwrap()
}

fn plant(observe: bool) -> LabSession {
    let scene: SceneDefinition = parse_json(SCENE).unwrap();
    let e = exercise();
    lab_model::validate_exercise(&e, &scene).unwrap();
    let mut s = LabSession::new(
        scene,
        e.starter_station.clone().unwrap(),
        PlatformProfile::generic(),
        SessionConfig::default(),
    )
    .unwrap();
    if observe {
        s.configure_sequence(e.sequence.clone().unwrap(), e.evidence.clone())
            .unwrap();
    }
    s.download(SOLUTION, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    s
}

fn run(s: &mut LabSession, ms: u64) {
    for _ in 0..ms * 1000 / s.config().cycle_us {
        s.step_cycle().unwrap();
    }
}

fn press(s: &mut LabSession, button: &str) {
    s.operate(OperatorAction::Press {
        component: button.into(),
    })
    .unwrap();
    run(s, 300);
    s.operate(OperatorAction::Release {
        component: button.into(),
    })
    .unwrap();
}

fn events(s: &LabSession) -> Vec<SequenceEvent> {
    s.events()
        .iter()
        .filter_map(|(_, e)| match &e.kind {
            LabEventKind::Sequence { event } => Some(event.clone()),
            _ => None,
        })
        .collect()
}

fn state(s: &LabSession) -> S {
    s.sequence().unwrap().status().state
}

fn active(s: &LabSession) -> String {
    s.sequence()
        .unwrap()
        .status()
        .active
        .as_ref()
        .map_or_else(String::new, ToString::to_string)
}

fn run_until(s: &mut LabSession, step: &str, limit_ms: u64) -> u64 {
    let start = s.now().as_micros();
    while active(s) != step {
        assert!(
            s.now().as_micros() - start < limit_ms * 1000,
            "{step} not reached in {limit_ms} ms (at {})",
            active(s)
        );
        s.step_cycle().unwrap();
    }
    s.now().as_micros() - start
}

#[test]
fn the_tank_batch_runs_through_every_step_with_real_process_feedback() {
    let mut s = plant(true);
    s.step_cycle().unwrap();
    assert_eq!(state(&s), S::Idle);
    press(&mut s, "start");
    // The valve needs its travel time before the open switch closes.
    let opened = run_until(&mut s, "fill", 3000);
    assert!(opened >= 1_000_000, "opened after {opened} µs");
    run_until(&mut s, "close-inlet", 20_000);
    run_until(&mut s, "drain", 3000);
    run_until(&mut s, "complete", 30_000);
    assert_eq!(state(&s), S::Completed);
    let all = events(&s);
    let codes: Vec<&str> = all.iter().map(|e| e.kind.code()).collect();
    assert_eq!(
        codes,
        [
            "runStarted",
            "transitionCompleted",
            "sequenceStarted",
            "transitionCompleted",
            "transitionCompleted",
            "transitionCompleted",
            "transitionCompleted",
            "sequenceCompleted"
        ]
    );
    let transitions: Vec<&str> = all
        .iter()
        .filter_map(|e| e.transition.as_ref().map(|t| t.as_str()))
        .collect();
    assert_eq!(transitions, ["start", "opened", "full", "closed", "empty"]);
    // Each transition was judged on the inputs the deciding scan read.
    for e in all.iter().filter(|e| e.kind == K::TransitionCompleted) {
        assert!(!e.values.is_empty());
    }
    assert!(all.windows(2).all(|w| w[0].seq < w[1].seq));
    assert!(s.alarms().alarms().iter().all(|a| !a.state.is_active()));
}

#[test]
fn a_stuck_inlet_valve_times_out_once_with_one_alarm_in_the_same_cycle() {
    let mut s = plant(true);
    s.inject_fault(&FaultInjection {
        id: lab_model::Identifier::new("stuck").unwrap(),
        fault: lab_model::fault::FaultSpec::ValveStuck {
            component: lab_model::Identifier::new("inlet").unwrap(),
            position: lab_model::fault::StuckPosition::Closed,
        },
    })
    .unwrap();
    s.step_cycle().unwrap();
    press(&mut s, "start");
    run(&mut s, 7000);
    assert_eq!(active(&s), "open-inlet");
    assert_eq!(state(&s), S::Faulted);
    let status = s.sequence().unwrap().status();
    assert_eq!(status.fault, Some(FaultReason::StepTimeout));
    assert_eq!(status.timeout_us, Some(5_000_000));
    assert!(status.elapsed_us >= 5_000_000);
    let all = events(&s);
    let activated = all
        .iter()
        .find(|e| e.transition.as_ref().is_some_and(|t| t.as_str() == "start"))
        .unwrap()
        .time_us;
    let timeouts: Vec<&SequenceEvent> = all.iter().filter(|e| e.kind == K::StepTimeout).collect();
    assert_eq!(timeouts.len(), 1);
    assert_eq!(
        timeouts[0].time_us - activated,
        5_000_000,
        "world time, exact"
    );
    // The waiting transition shows the missing feedback.
    let opened = status
        .transitions
        .iter()
        .find(|t| t.id.as_str() == "opened")
        .unwrap();
    assert!(opened.from_active);
    assert_eq!(opened.condition.satisfied, Some(false));
    // One alarm activation, raised in the cycle of the timeout and logged after it.
    let alarm = s.alarms().get(TIMEOUT_ALARM).unwrap();
    assert_eq!(alarm.state, AlarmState::ActiveUnacknowledged);
    let activations = s
        .alarms()
        .history()
        .filter(|e| e.definition.id == TIMEOUT_ALARM && e.transition == AlarmTransition::Activated)
        .count();
    assert_eq!(activations, 1);
    let log: Vec<(u64, &str)> = s
        .events()
        .iter()
        .filter_map(|(_, e)| match &e.kind {
            LabEventKind::Sequence { event } if event.kind == K::StepTimeout => {
                Some((e.time_us, "timeout"))
            }
            LabEventKind::Alarm { id, .. } if id == TIMEOUT_ALARM => Some((e.time_us, "alarm")),
            _ => None,
        })
        .collect();
    assert_eq!(log[0].1, "timeout");
    assert_eq!(log[1].1, "alarm");
    assert_eq!(log[0].0, log[1].0, "same world time, stable order");
    // Returning to idle (Stop) clears the fault and the alarm's condition.
    press(&mut s, "stop");
    assert_eq!(state(&s), S::Idle);
    assert!(!s.alarms().get(TIMEOUT_ALARM).unwrap().condition);
}

#[test]
fn hold_stops_the_plant_through_the_program_and_resume_continues() {
    let mut s = plant(true);
    s.step_cycle().unwrap();
    press(&mut s, "start");
    run_until(&mut s, "fill", 3000);
    run(&mut s, 2000);
    press(&mut s, "hold");
    assert_eq!(state(&s), S::Held);
    let paused = s.sequence().unwrap().status().elapsed_us;
    run(&mut s, 3000);
    assert_eq!(active(&s), "fill");
    assert_eq!(s.sequence().unwrap().status().elapsed_us, paused);
    let valve = s
        .signals()
        .by_path("process.inlet.positionPermille")
        .unwrap();
    let position = s
        .latest()
        .unwrap()
        .process_after
        .iter()
        .find(|(id, _)| *id == valve)
        .unwrap()
        .1;
    assert_eq!(
        position,
        SignalValue::Count(0),
        "the program closed the valve"
    );
    press(&mut s, "start");
    assert_eq!(state(&s), S::Running);
    run_until(&mut s, "complete", 40_000);
    let codes: Vec<&str> = events(&s).iter().map(|e| e.kind.code()).collect();
    assert!(codes.contains(&"held") && codes.contains(&"resumed"));
    assert!(!events(&s).iter().any(|e| e.kind.is_fault()));
}

#[test]
fn stop_aborts_through_the_interlock_reaction() {
    let mut s = plant(true);
    s.step_cycle().unwrap();
    press(&mut s, "start");
    run_until(&mut s, "fill", 3000);
    press(&mut s, "stop");
    assert_eq!(state(&s), S::Idle);
    let aborted = events(&s)
        .into_iter()
        .find(|e| e.kind == K::SequenceAborted)
        .unwrap();
    assert_eq!(aborted.interlock.unwrap().as_str(), "stop-pressed");
    assert!(!events(&s).iter().any(|e| e.kind.is_fault()));
}

#[test]
fn every_plc_start_begins_a_new_run_without_a_false_deviation() {
    let mut s = plant(true);
    s.step_cycle().unwrap();
    press(&mut s, "start");
    run_until(&mut s, "fill", 3000);
    let last_run = |s: &LabSession| {
        events(s)
            .into_iter()
            .rfind(|e| matches!(e.kind, K::RunStarted { .. }))
            .unwrap()
    };
    s.restart(StartKind::Warm).unwrap();
    s.step_cycle().unwrap();
    let run_start = last_run(&s);
    assert_eq!(
        run_start.kind,
        K::RunStarted {
            start: RunStart::Warm
        }
    );
    assert_eq!(run_start.time_us, s.now().as_micros());
    // Step is not retained: a warm start initializes it, the run begins in idle. The
    // valve is still open from filling, so the start interlock is active at once.
    assert_eq!(state(&s), S::Idle);
    assert_eq!(
        events(&s).pop().unwrap().interlock.unwrap().as_str(),
        "inlet-not-closed"
    );
    s.restart(StartKind::Cold).unwrap();
    s.step_cycle().unwrap();
    assert_eq!(
        last_run(&s).kind,
        K::RunStarted {
            start: RunStart::Cold
        }
    );
    // STOP: no scans, the sequence is stopped.
    s.set_plc_switch(PlcModeRequest::Stop).unwrap();
    s.step_cycle().unwrap();
    assert_eq!(state(&s), S::Stopped);
    assert!(!events(&s).iter().any(|e| e.kind.is_fault()));
}

#[test]
fn a_forced_input_reaches_the_monitor_as_the_program_reads_it() {
    let mut s = plant(true);
    s.step_cycle().unwrap();
    let start = s.signals().by_path("plc.StartPB").unwrap();
    s.force(start, Some(SignalValue::Bool(true))).unwrap();
    s.step_cycle().unwrap();
    s.step_cycle().unwrap();
    assert_eq!(active(&s), "open-inlet");
    let started = events(&s)
        .into_iter()
        .find(|e| e.kind == K::TransitionCompleted)
        .unwrap();
    assert!(started.values.iter().any(|v| v.signal.as_str() == "start"
        && v.value == Some(grading_core::EvidenceValue::Bool(true))));
}

#[test]
fn observing_never_changes_the_plant() {
    let mut watched = plant(true);
    let mut plain = plant(false);
    for s in [&mut watched, &mut plain] {
        s.step_cycle().unwrap();
        press(s, "start");
        run(s, 6000);
        press(s, "hold");
        run(s, 1000);
        press(s, "start");
        run(s, 25_000);
    }
    let (a, b) = (watched.latest().unwrap(), plain.latest().unwrap());
    assert_eq!(a.header, b.header);
    assert_eq!(a.committed, b.committed);
    assert_eq!(a.applied, b.applied);
    assert_eq!(a.process_after, b.process_after);
    assert!(plain.sequence().is_none());
    assert!(
        plain
            .alarms()
            .alarms()
            .iter()
            .all(|a| !a.definition.id.starts_with("sequence."))
    );
}

#[test]
fn a_world_reset_keeps_observing_with_a_new_history() {
    let mut s = plant(true);
    s.step_cycle().unwrap();
    press(&mut s, "start");
    run(&mut s, 1000);
    assert!(s.sequence().unwrap().total_events() > 0);
    s.reset_world().unwrap();
    assert_eq!(s.sequence().unwrap().total_events(), 0);
    s.step_cycle().unwrap();
    assert_eq!(state(&s), S::Idle);
    assert!(s.alarms().get(TIMEOUT_ALARM).is_some());
}

#[test]
fn a_program_without_the_step_variable_is_unbound() {
    let mut s = plant(true);
    s.download(
        "PROGRAM Main\nVAR_OUTPUT\n    InletValve : BOOL;\nEND_VAR\nInletValve := FALSE;\nEND_PROGRAM\n",
        2,
        None,
    )
    .unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    s.step_cycle().unwrap();
    assert_eq!(state(&s), S::Unbound);
    assert!(events(&s).iter().all(|e| !e.kind.is_fault()));
}

#[test]
fn an_undefined_jump_is_a_deviation_with_an_alarm() {
    let mut s = plant(true);
    s.download(&SOLUTION.replace("Step := 1;", "Step := 3;"), 2, None)
        .unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    s.step_cycle().unwrap();
    press(&mut s, "start");
    assert_eq!(state(&s), S::Faulted);
    assert_eq!(
        s.sequence().unwrap().status().fault,
        Some(FaultReason::Deviation(Deviation::UndefinedTransition))
    );
    assert!(
        s.alarms()
            .get("sequence.tank-batch.deviation")
            .unwrap()
            .state
            .is_active()
    );
}

#[test]
fn assessment_is_isolated_repeatable_and_needs_a_working_sequence() {
    let scene: SceneDefinition = parse_json(SCENE).unwrap();
    let e = exercise();
    let station = e.starter_station.clone().unwrap();
    let grade = |program: &str| match assess(
        &e,
        &scene,
        &station,
        &PlatformProfile::generic(),
        program,
        &[],
        None,
    )
    .unwrap()
    {
        AssessmentOutcome::Graded(r) => r,
        other => panic!("{other:?}"),
    };
    let first = grade(SOLUTION);
    assert!(first.passed, "{first:?}");
    assert_eq!(first, grade(SOLUTION));
    let starter = grade(&e.starter_program);
    assert!(!starter.passed);
    // A program that skips the documented open-inlet step fails conformance only.
    let skipping = grade(&SOLUTION.replace(
        "IF InletOpen THEN\n                Step := 2;",
        "IF TRUE THEN\n                Step := 2;",
    ));
    let failed: Vec<&str> = skipping
        .criteria
        .iter()
        .filter(|c| !c.passed)
        .map(|c| c.id.as_str())
        .collect();
    assert!(failed.contains(&"conforms"), "{failed:?}");
}

#[test]
fn the_stalled_sequence_lab_fails_until_the_valve_is_repaired() {
    let scene: SceneDefinition = parse_json(SCENE).unwrap();
    let lab: ExerciseDefinition = parse_json(include_str!(
        "../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/ts-tank-sequence-stalls.json"
    ))
    .unwrap();
    lab_model::validate_exercise(&lab, &scene).unwrap();
    let t = lab.troubleshooting.as_ref().unwrap();
    let station = lab.starter_station.clone().unwrap();
    let program = t.loaded_program.as_deref().unwrap();
    assert_eq!(program, SOLUTION, "the plant runs the working program");
    let verify = |faults: &[FaultInjection]| match assess(
        &lab,
        &scene,
        &station,
        &PlatformProfile::generic(),
        program,
        faults,
        Some(&t.verification),
    )
    .unwrap()
    {
        AssessmentOutcome::Graded(r) => r,
        other => panic!("{other:?}"),
    };
    let broken = verify(&t.faults);
    assert!(!broken.passed);
    assert!(
        broken
            .criteria
            .iter()
            .any(|c| c.id == "conforms" && c.detail.contains("stepTimeout")),
        "{broken:?}"
    );
    let repaired = verify(&[]);
    assert!(repaired.passed, "{repaired:?}");
}
