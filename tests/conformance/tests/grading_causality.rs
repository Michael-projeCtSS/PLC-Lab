//! Grading regression: the evidence used for grading is causal. For `Lamp1 := NOT Button1`
//! the committed output of every cycle equals the negation of the input image *that same
//! cycle* read, never a later sample. A grading rule written against that relation passes
//! for the correct program and fails for the non-inverting one.
use grading_core::{CycleFacts, CycleOutcome, EvidenceValue, GradingEvidence, evaluate_rule};
use lab_engine::{AssessmentSetup, SessionConfig, run_scenario};
use lab_model::{
    exercise::{EvidenceSignal, Rule, Scenario},
    parse_json,
    profile::PlatformProfile,
    scene::SceneDefinition,
};

const IO: &str = include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/io-basics.json");
const MOTOR: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/motor-start-stop.json");

fn program(body: &str) -> String {
    format!(
        "PROGRAM Main VAR_INPUT Button1 : BOOL; END_VAR VAR_OUTPUT Lamp1 : BOOL; END_VAR {body} END_PROGRAM"
    )
}

fn evidence(scene: &SceneDefinition, body: &str) -> GradingEvidence {
    let station = scene.reference_station.clone();
    let profile = PlatformProfile::generic();
    let source = program(body);
    let setup = AssessmentSetup {
        scene,
        station: &station,
        profile: &profile,
        program: &source,
        faults: &[],
        config: SessionConfig::default(),
    };
    // Press S1 twice with releases in between, so both input values occur repeatedly.
    let scenario: Scenario = serde_json::from_value(serde_json::json!({
        "id": "toggle", "title": "Press and release S1", "durationMs": 4000,
        "actions": [
            { "atMs": 0, "action": { "type": "setPlcMode", "mode": "run" } },
            { "atMs": 500, "action": { "type": "press", "component": "pb1" } },
            { "atMs": 1250, "action": { "type": "release", "component": "pb1" } },
            { "atMs": 2000, "action": { "type": "press", "component": "pb1" } },
            { "atMs": 2050, "action": { "type": "release", "component": "pb1" } },
            { "atMs": 3000, "action": { "type": "press", "component": "pb1" } }
        ]
    }))
    .unwrap();
    let signals: Vec<EvidenceSignal> = serde_json::from_value(serde_json::json!([
        { "name": "button", "source": { "type": "plcTag", "tag": "Button1" } },
        { "name": "lamp", "source": { "type": "plcTag", "tag": "Lamp1" } }
    ]))
    .unwrap();
    run_scenario(&setup, &scenario, &signals).unwrap()
}

fn inverted() -> Rule {
    serde_json::from_value(serde_json::json!({
        "type": "always", "fromMs": 0, "condition": { "type": "any", "of": [
            { "type": "all", "of": [
                { "type": "compare", "signal": "button", "layer": "inputs", "op": "eq", "value": true },
                { "type": "compare", "signal": "lamp", "layer": "outputs", "op": "eq", "value": false }
            ]},
            { "type": "all", "of": [
                { "type": "compare", "signal": "button", "layer": "inputs", "op": "eq", "value": false },
                { "type": "compare", "signal": "lamp", "layer": "outputs", "op": "eq", "value": true }
            ]}
        ]}
    }))
    .unwrap()
}

#[test]
fn inverter_output_is_the_negation_of_the_same_cycles_scan_input() {
    let scene: SceneDefinition = parse_json(IO).unwrap();
    let run = evidence(&scene, "Lamp1 := NOT Button1;");
    let executed: Vec<_> = run
        .cycles
        .iter()
        .filter(|c| c.outcome == Some(CycleOutcome::Scanned))
        .collect();
    assert!(executed.len() >= 30, "{} executed cycles", executed.len());
    let mut seen = [false; 2];
    for c in &executed {
        let EvidenceValue::Bool(input) = c.inputs["button"] else {
            panic!("cycle {}: Button1 is not BOOL", c.cycle)
        };
        let EvidenceValue::Bool(output) = c.outputs["lamp"] else {
            panic!("cycle {}: Lamp1 is not BOOL", c.cycle)
        };
        assert_eq!(
            output, !input,
            "cycle {}: Lamp1 must be NOT of the input this cycle read",
            c.cycle
        );
        seen[usize::from(input)] = true;
    }
    assert_eq!(seen, [true, true], "both input values must occur");
    // A 50 ms press shorter than one 100 ms cycle may be missed, but whatever the scan
    // saw, its output answers exactly that.
    let (passed, detail) = evaluate_rule(&inverted(), &run);
    assert!(passed, "{detail}");
}

#[test]
fn non_inverting_program_fails_the_same_causal_rule() {
    let scene: SceneDefinition = parse_json(IO).unwrap();
    let run = evidence(&scene, "Lamp1 := Button1;");
    let (passed, _) = evaluate_rule(&inverted(), &run);
    assert!(!passed);
}

#[test]
fn grading_runs_are_deterministic() {
    let scene: SceneDefinition = parse_json(IO).unwrap();
    let a = evidence(&scene, "Lamp1 := NOT Button1;");
    let b = evidence(&scene, "Lamp1 := NOT Button1;");
    assert_eq!(a, b);
}

fn bool_in(layer: &std::collections::BTreeMap<String, EvidenceValue>, name: &str) -> bool {
    match layer.get(name) {
        Some(EvidenceValue::Bool(b)) => *b,
        other => panic!("{name}: {other:?}"),
    }
}
fn cycle_at(run: &GradingEvidence, start_ms: u64) -> &CycleFacts {
    run.cycles
        .iter()
        .find(|c| c.start_us == start_ms * 1_000)
        .unwrap_or_else(|| panic!("no cycle starts at {start_ms} ms"))
}

/// One cycle written out: S1 is pressed at 2000 ms and released at 2050 ms, inside the
/// cycle 2000..2100 ms. The scan read TRUE, so the committed output is FALSE; the input
/// staged for the next scan is already FALSE. Grading judges the cycle by what it read.
#[test]
fn single_cycle_proof_scan_input_committed_output_and_next_input() {
    let scene: SceneDefinition = parse_json(IO).unwrap();
    let run = evidence(&scene, "Lamp1 := NOT Button1;");
    let c = cycle_at(&run, 2000);
    let (scan, committed, next) = (
        bool_in(&c.inputs, "button"),
        bool_in(&c.outputs, "lamp"),
        bool_in(&c.next, "button"),
    );
    println!(
        "cycle {} [{}..{} us]: scan input Button1 = {scan}; committed Lamp1 = {committed}; next staged Button1 = {next}",
        c.cycle, c.start_us, c.end_us
    );
    assert_eq!(c.outcome, Some(CycleOutcome::Scanned));
    assert!(scan, "the press at 2000 ms is in this scan's input image");
    assert!(!committed, "Lamp1 = NOT the scanned value");
    assert!(
        !next,
        "the release at 2050 ms is staged for the next scan only"
    );
    // The grading rule sees the same pairing: no cycle pairs an output with a later input.
    let (passed, detail) = evaluate_rule(&inverted(), &run);
    assert!(passed, "{detail}");
}

/// The process chain on the motor station: the scan reads Start, the program commits the
/// contactor command, the field device pulls in during the cycle, and the auxiliary
/// contact appears only in the input staged for the next scan.
#[test]
fn scan_input_command_process_effect_and_next_sensor_value() {
    let scene: SceneDefinition = parse_json(MOTOR).unwrap();
    let station = scene.reference_station.clone();
    let profile = PlatformProfile::generic();
    let source = "PROGRAM Main
VAR_INPUT StartPB : BOOL; StopPB : BOOL; MotorFeedback : BOOL; OverloadOK : BOOL; END_VAR
VAR_OUTPUT MotorContactor : BOOL; RunLamp : BOOL; END_VAR
MotorContactor := (StartPB OR MotorContactor) AND StopPB AND OverloadOK;
RunLamp := MotorFeedback;
END_PROGRAM";
    let setup = AssessmentSetup {
        scene: &scene,
        station: &station,
        profile: &profile,
        program: source,
        faults: &[],
        config: SessionConfig::default(),
    };
    let scenario: Scenario = serde_json::from_value(serde_json::json!({
        "id": "start", "title": "Press Start", "durationMs": 1500,
        "actions": [
            { "atMs": 0, "action": { "type": "setPlcMode", "mode": "run" } },
            { "atMs": 500, "action": { "type": "press", "component": "start" } },
            { "atMs": 800, "action": { "type": "release", "component": "start" } }
        ]
    }))
    .unwrap();
    let signals: Vec<EvidenceSignal> = serde_json::from_value(serde_json::json!([
        { "name": "start", "source": { "type": "plcTag", "tag": "StartPB" } },
        { "name": "contactor", "source": { "type": "plcTag", "tag": "MotorContactor" } },
        { "name": "coil", "source": { "type": "component", "component": "motor", "quantity": "command" } },
        { "name": "feedback", "source": { "type": "plcTag", "tag": "MotorFeedback" } },
        { "name": "lamp", "source": { "type": "plcTag", "tag": "RunLamp" } }
    ]))
    .unwrap();
    let run = run_scenario(&setup, &scenario, &signals).unwrap();
    let before = cycle_at(&run, 400);
    assert!(!bool_in(&before.inputs, "start") && !bool_in(&before.outputs, "contactor"));
    let c = cycle_at(&run, 500);
    println!(
        "cycle {}: scan StartPB = {}, MotorFeedback = {}; committed MotorContactor = {}; process motor.command = {}; next staged MotorFeedback = {}",
        c.cycle,
        bool_in(&c.inputs, "start"),
        bool_in(&c.inputs, "feedback"),
        bool_in(&c.outputs, "contactor"),
        bool_in(&c.process, "coil"),
        bool_in(&c.next, "feedback"),
    );
    assert!(bool_in(&c.inputs, "start"), "1. the scan reads Start");
    assert!(
        bool_in(&c.outputs, "contactor"),
        "2. the program commits the command"
    );
    assert!(
        bool_in(&c.process, "coil"),
        "3. the contactor coil is energized"
    );
    assert!(
        !bool_in(&c.inputs, "feedback"),
        "the scan could not see the feedback yet"
    );
    assert!(
        !bool_in(&c.outputs, "lamp"),
        "so RunLamp stays off in this cycle"
    );
    assert!(
        bool_in(&c.next, "feedback"),
        "4. the pulled-in contactor is staged for the next scan"
    );
    let after = cycle_at(&run, 600);
    assert!(bool_in(&after.inputs, "feedback") && bool_in(&after.outputs, "lamp"));
}
