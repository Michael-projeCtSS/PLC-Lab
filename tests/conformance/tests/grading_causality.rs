//! Grading regression: the evidence used for grading is causal. For `Lamp1 := NOT Button1`
//! the committed output of every cycle equals the negation of the input image *that same
//! cycle* read, never a later sample. A grading rule written against that relation passes
//! for the correct program and fails for the non-inverting one.
use grading_core::{EvidenceValue, GradingEvidence, evaluate_rule};
use lab_engine::{AssessmentSetup, SessionConfig, run_scenario};
use lab_model::{
    exercise::{EvidenceSignal, Rule, Scenario},
    parse_json,
    profile::PlatformProfile,
    scene::SceneDefinition,
};

const IO: &str = include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/io-basics.json");

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
    let executed: Vec<_> = run.cycles.iter().filter(|c| c.outcome.is_some()).collect();
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
