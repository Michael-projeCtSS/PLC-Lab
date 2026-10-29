//! Commissioning checklist over the digital I/O commissioning bench and its labs.
use lab_engine::{CheckState, Checklist, LabSession, OperatorAction, SessionConfig};
use lab_model::{
    exercise::{ExerciseDefinition, PlcModeRequest},
    fault::FaultInjection,
    parse_json,
    profile::PlatformProfile,
    scene::SceneDefinition,
};

const SCENE: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/dio-commissioning.json");
const LABS: [&str; 6] = [
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-dio-input-supply.json"),
    include_str!(
        "../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-dio-guard-contact.json"
    ),
    include_str!(
        "../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-dio-swapped-buttons.json"
    ),
    include_str!(
        "../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-dio-swapped-outputs.json"
    ),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-dio-feedback.json"),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-dio-wrong-slot.json"),
];

fn lab(index: usize) -> ExerciseDefinition {
    parse_json(LABS[index]).unwrap()
}

/// A bench with the lab's program loaded, the PLC in RUN and the lab's faults injected.
fn bench(faulty: Option<usize>) -> LabSession {
    let scene: SceneDefinition = parse_json(SCENE).unwrap();
    let station = scene.reference_station.clone();
    let mut s = LabSession::new(
        scene,
        station,
        PlatformProfile::generic(),
        SessionConfig::default(),
    )
    .unwrap();
    let exercise = lab(faulty.unwrap_or(0));
    if let Some(t) = faulty.and(exercise.troubleshooting.as_ref()) {
        for f in &t.faults {
            s.inject_fault(&FaultInjection {
                id: f.id.clone(),
                fault: f.fault.clone(),
            })
            .unwrap();
        }
    }
    s.download(&exercise.starter_program, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    s
}

fn run(s: &mut LabSession, ms: u64) {
    let cycles = ms * 1000 / s.config().cycle_us;
    for _ in 0..cycles.max(1) {
        s.step_cycle().unwrap();
    }
}

fn act(s: &mut LabSession, action: OperatorAction, ms: u64) {
    s.operate(action).unwrap();
    run(s, ms);
}

fn press(s: &mut LabSession, component: &str) {
    act(
        s,
        OperatorAction::Press {
            component: component.into(),
        },
        300,
    );
    act(
        s,
        OperatorAction::Release {
            component: component.into(),
        },
        300,
    );
}

fn present(s: &mut LabSession, component: &str, present: bool) {
    act(
        s,
        OperatorAction::SetPresence {
            component: component.into(),
            present,
        },
        300,
    );
}

/// The commissioning engineer's I/O test: every device, then a start/stop run.
fn commission(s: &mut LabSession) {
    present(s, "part", true);
    present(s, "part", false);
    present(s, "guard", false);
    present(s, "guard", true);
    press(s, "stop");
    press(s, "start");
    run(s, 4_000);
    press(s, "stop");
    run(s, 1_500);
}

fn state(list: &Checklist, category: &str, item: &str) -> CheckState {
    list.categories
        .iter()
        .find(|c| c.id == category)
        .and_then(|c| c.items.iter().find(|i| i.id == item))
        .unwrap_or_else(|| panic!("no check {category}/{item}"))
        .state
}

fn findings(list: &Checklist, category: &str, item: &str) -> String {
    list.categories
        .iter()
        .find(|c| c.id == category)
        .and_then(|c| c.items.iter().find(|i| i.id == item))
        .map(|i| i.findings.join("\n"))
        .unwrap()
}

#[test]
fn a_fresh_bench_shows_its_state_and_waits_for_the_io_tests() {
    let s = bench(None);
    let list = s.commissioning();
    let ids: Vec<&str> = list.categories.iter().map(|c| c.id).collect();
    assert_eq!(
        ids,
        [
            "power", "hardware", "network", "inputs", "outputs", "drives", "process"
        ]
    );
    assert_eq!(state(&list, "power", "io-supply"), CheckState::Ok);
    assert_eq!(state(&list, "hardware", "module-types"), CheckState::Ok);
    assert_eq!(state(&list, "hardware", "slot-order"), CheckState::Ok);
    assert_eq!(state(&list, "hardware", "plc-state"), CheckState::Ok);
    assert_eq!(
        state(&list, "inputs", "sensor-operation"),
        CheckState::Untested
    );
    assert_eq!(state(&list, "inputs", "contact-type"), CheckState::Untested);
    assert_eq!(state(&list, "outputs", "physical"), CheckState::Untested);
    assert_eq!(state(&list, "outputs", "feedback"), CheckState::Untested);
    for category in ["network", "drives"] {
        let c = list.categories.iter().find(|c| c.id == category).unwrap();
        assert_eq!(c.state(), CheckState::NotApplicable, "{category}");
    }
}

#[test]
fn a_correct_installation_passes_every_check_after_the_io_tests() {
    let mut s = bench(None);
    commission(&mut s);
    let list = s.commissioning();
    for c in &list.categories {
        for i in &c.items {
            assert!(
                matches!(i.state, CheckState::Ok | CheckState::NotApplicable),
                "{}/{}: {:?} {:?}",
                c.id,
                i.id,
                i.state,
                i.findings
            );
        }
    }
    assert!(
        findings(&list, "inputs", "sensor-operation")
            .contains("Start S1: %IX0.0 (StartPB) follows")
    );
}

#[test]
fn each_installation_fault_shows_where_a_commissioning_engineer_finds_it() {
    let expected: [(&str, &str, CheckState, &str); 6] = [
        ("power", "io-supply", CheckState::Fault, "no 24 V field"),
        (
            "inputs",
            "contact-type",
            CheckState::Attention,
            "Guard door B2: drawn normally open",
        ),
        (
            "inputs",
            "sensor-operation",
            CheckState::Fault,
            "Start S1: operating it changed %IX0.1 (StopPB) instead of %IX0.0",
        ),
        ("outputs", "actuator", CheckState::Fault, "did not respond"),
        (
            "outputs",
            "feedback",
            CheckState::Fault,
            "feedback %IX0.4 (MotorFeedback) never changed",
        ),
        (
            "hardware",
            "slot-order",
            CheckState::Fault,
            "no module found",
        ),
    ];
    for (index, (category, item, want, text)) in expected.into_iter().enumerate() {
        let mut s = bench(Some(index));
        commission(&mut s);
        let list = s.commissioning();
        assert_eq!(
            state(&list, category, item),
            want,
            "lab {index} {category}/{item}: {}",
            findings(&list, category, item)
        );
        assert!(
            findings(&list, category, item).contains(text),
            "lab {index}: {}",
            findings(&list, category, item)
        );
    }
}

#[test]
fn findings_are_observations_and_never_name_the_injected_fault() {
    for index in 0..LABS.len() {
        let exercise = lab(index);
        let t = exercise.troubleshooting.as_ref().unwrap();
        let mut s = bench(Some(index));
        commission(&mut s);
        let text: String = s
            .commissioning()
            .categories
            .iter()
            .flat_map(|c| c.items.iter())
            .flat_map(|i| i.findings.iter().map(String::as_str).chain([i.how]))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        for secret in t
            .root_causes
            .iter()
            .map(ToString::to_string)
            .chain(t.repairs.iter().map(|r| r.label.clone()))
            .chain(t.diagnosis_options.iter().map(|o| o.label.clone()))
        {
            assert!(
                !text.contains(&secret.to_lowercase()),
                "lab {index} reveals {secret}"
            );
        }
    }
}

#[test]
fn a_repair_starts_the_io_tests_again() {
    let mut s = bench(Some(2));
    commission(&mut s);
    assert_eq!(
        state(&s.commissioning(), "inputs", "sensor-operation"),
        CheckState::Fault
    );
    let id = lab(2).troubleshooting.unwrap().root_causes[0].clone();
    assert_eq!(s.repair("Swap the wires back", &[id]).unwrap(), 1);
    assert_eq!(
        state(&s.commissioning(), "inputs", "sensor-operation"),
        CheckState::Untested
    );
    commission(&mut s);
    assert_eq!(
        state(&s.commissioning(), "inputs", "sensor-operation"),
        CheckState::Ok
    );
}

#[test]
fn the_checklist_is_deterministic() {
    let mut a = bench(Some(4));
    let mut b = bench(Some(4));
    commission(&mut a);
    commission(&mut b);
    assert_eq!(a.commissioning(), b.commissioning());
}

const ANALOG_SCENE: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/analog-commissioning.json");
const ANALOG_LABS: [&str; 5] = [
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-ai-loop-break.json"),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-ai-loop-supply.json"),
    include_str!(
        "../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-ai-voltage-as-current.json"
    ),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-ai-zero-twenty.json"),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-ai-offset.json"),
];

/// The analog bench with a lab's starter configuration and faults (or the documented one).
fn analog_bench(faulty: Option<usize>) -> LabSession {
    let scene: SceneDefinition = parse_json(ANALOG_SCENE).unwrap();
    let exercise: ExerciseDefinition = parse_json(ANALOG_LABS[faulty.unwrap_or(0)]).unwrap();
    let station = faulty
        .and(exercise.starter_station.clone())
        .unwrap_or_else(|| scene.reference_station.clone());
    let mut s = LabSession::new(
        scene,
        station,
        PlatformProfile::generic(),
        SessionConfig::default(),
    )
    .unwrap();
    if let Some(t) = faulty.and(exercise.troubleshooting.as_ref()) {
        for f in &t.faults {
            s.inject_fault(&FaultInjection {
                id: f.id.clone(),
                fault: f.fault.clone(),
            })
            .unwrap();
        }
    }
    s.download(&exercise.starter_program, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    run(&mut s, 500);
    s
}

#[test]
fn a_correct_analog_installation_matches_the_drawing_and_the_sight_glass() {
    let s = analog_bench(None);
    let list = s.commissioning();
    for item in ["analog-signal", "analog-raw", "scaling"] {
        assert_eq!(
            state(&list, "inputs", item),
            CheckState::Ok,
            "{item}: {}",
            findings(&list, "inputs", item)
        );
    }
    let raw = findings(&list, "inputs", "analog-raw");
    assert!(raw.contains("7.20 mA"), "20 % of a 4-20 mA loop: {raw}");
    assert!(raw.contains("2.00 V"), "20 % of 0-10 V: {raw}");
    assert!(
        findings(&list, "inputs", "scaling").contains("= 20000 ml; sight glass 20000 ml"),
        "{}",
        findings(&list, "inputs", "scaling")
    );
}

#[test]
fn each_analog_fault_shows_in_the_analog_checks() {
    let expected: [(&str, CheckState, &str); 5] = [
        ("analog-raw", CheckState::Fault, "0.00 mA"),
        (
            "analog-signal",
            CheckState::Fault,
            "2-wire transmitter, but",
        ),
        ("analog-signal", CheckState::Fault, "drawn 0-10 V"),
        ("analog-signal", CheckState::Fault, "drawn 4-20 mA"),
        ("scaling", CheckState::Attention, "differs by 10000 ml"),
    ];
    for (index, (item, want, text)) in expected.into_iter().enumerate() {
        let s = analog_bench(Some(index));
        let list = s.commissioning();
        let found = findings(&list, "inputs", item);
        assert_eq!(state(&list, "inputs", item), want, "lab {index}: {found}");
        assert!(found.contains(text), "lab {index}: {found}");
    }
}

const DRIVE_SCENE: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/drive-commissioning.json");
const DRIVE_LABS: [&str; 5] = [
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-vfd-local.json"),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-vfd-trip.json"),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-vfd-enable.json"),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-vfd-keypad.json"),
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/cm-vfd-no-supply.json"),
];

/// The drive bench in RUN after Start was pressed, with a lab's faults (or none).
fn drive_bench(faulty: Option<usize>) -> LabSession {
    let scene: SceneDefinition = parse_json(DRIVE_SCENE).unwrap();
    let station = scene.reference_station.clone();
    let exercise: ExerciseDefinition = parse_json(DRIVE_LABS[faulty.unwrap_or(0)]).unwrap();
    let mut s = LabSession::new(
        scene,
        station,
        PlatformProfile::generic(),
        SessionConfig::default(),
    )
    .unwrap();
    if let Some(t) = faulty.and(exercise.troubleshooting.as_ref()) {
        for f in &t.faults {
            s.inject_fault(&FaultInjection {
                id: f.id.clone(),
                fault: f.fault.clone(),
            })
            .unwrap();
        }
    }
    s.download(&exercise.starter_program, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    press(&mut s, "start");
    run(&mut s, 3_000);
    s
}

#[test]
fn a_correct_drive_installation_passes_the_drive_checks() {
    let list = drive_bench(None).commissioning();
    for item in [
        "ready",
        "control-place",
        "enable",
        "fault",
        "run-command",
        "reference",
        "speed",
    ] {
        assert_eq!(
            state(&list, "drives", item),
            CheckState::Ok,
            "{item}: {}",
            findings(&list, "drives", item)
        );
    }
    assert!(findings(&list, "drives", "speed").contains("running at 50.0 % speed"));
    assert!(findings(&list, "drives", "reference").contains("50.0 % from the terminals"));
}

#[test]
fn each_drive_fault_shows_in_the_drive_checks() {
    let expected: [(&str, &str, CheckState, &str); 5] = [
        (
            "drives",
            "control-place",
            CheckState::Attention,
            "local control",
        ),
        (
            "drives",
            "fault",
            CheckState::Fault,
            "faulted (fault relay closed)",
        ),
        (
            "drives",
            "ready",
            CheckState::Fault,
            "not ready (run enable missing); the run command is active",
        ),
        (
            "drives",
            "reference",
            CheckState::Attention,
            "30.0 % from the keypad",
        ),
        (
            "power",
            "drive-supply",
            CheckState::Fault,
            "no mains supply",
        ),
    ];
    for (index, (category, item, want, text)) in expected.into_iter().enumerate() {
        let list = drive_bench(Some(index)).commissioning();
        let found = findings(&list, category, item);
        assert_eq!(state(&list, category, item), want, "lab {index}: {found}");
        assert!(found.contains(text), "lab {index}: {found}");
    }
}
