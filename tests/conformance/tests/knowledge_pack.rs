//! The shipped knowledge pack is valid, searchable, and every exercise in it is solvable:
//! practice exercises pass with their reference solution and fail with the starter
//! program; troubleshooting exercises pass once repaired and fail with the fault present.
use knowledge_core::{ArticleKind, Category, KnowledgeLibrary, PackStatus, SearchQuery};
use lab_engine::{AssessmentOutcome, assess};
use lab_model::exercise::ExerciseKind;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn library() -> KnowledgeLibrary {
    KnowledgeLibrary::load(&[root().join("knowledge")])
}

fn score(outcome: AssessmentOutcome) -> (bool, String) {
    match outcome {
        AssessmentOutcome::Graded(r) => (
            r.passed,
            r.criteria
                .iter()
                .filter(|c| !c.passed)
                .map(|c| format!("{}: {}", c.id, c.detail))
                .collect::<Vec<_>>()
                .join(" | "),
        ),
        other => (false, format!("{other:?}")),
    }
}

#[test]
fn pack_loads_without_errors_or_dangling_references() {
    let lib = library();
    let packs = lib.packs();
    assert_eq!(packs.len(), 1);
    let pack = &packs[0];
    assert_eq!(pack.status, PackStatus::Loaded, "{:#?}", pack.errors);
    assert!(pack.warnings.is_empty(), "{:#?}", pack.warnings);
    assert!(pack.counts.articles >= 74, "{:?}", pack.counts);
    assert!(pack.counts.lessons >= 20, "{:?}", pack.counts);
    assert!(pack.counts.exercises >= 17, "{:?}", pack.counts);
    assert!(pack.counts.profiles >= 9, "{:?}", pack.counts);
    for vendor in [
        "vendor.siemens",
        "vendor.rockwell",
        "vendor.beckhoff",
        "vendor.schneider",
        "vendor.mitsubishi",
        "vendor.omron",
        "vendor.phoenix-contact",
        "vendor.wago",
    ] {
        assert!(lib.article(vendor).is_some(), "{vendor}");
    }
    for protocol in [
        "modbus-tcp",
        "modbus-rtu",
        "profinet",
        "profibus-dp",
        "ethernet-ip",
        "ethercat",
        "opc-ua",
        "canopen",
        "io-link",
    ] {
        assert!(
            lib.article(&format!("protocol.{protocol}")).is_some(),
            "{protocol}"
        );
    }
    // Every reference in lessons and exercises resolves.
    for lesson in lib.lessons() {
        for r in &lesson.knowledge_refs {
            assert!(lib.title_of(r).is_some(), "lesson {} -> {r}", lesson.id);
        }
        for s in &lesson.sections {
            if let Some(e) = &s.exercise {
                assert!(
                    lib.exercise(e.as_str()).is_some(),
                    "lesson {} -> {e}",
                    lesson.id
                );
            }
            if let Some(scene) = &s.scene {
                assert!(
                    lib.scene(scene.as_str()).is_some(),
                    "lesson {} -> {scene}",
                    lesson.id
                );
            }
        }
    }
    for e in lib.exercises() {
        for r in &e.knowledge_refs {
            assert!(lib.title_of(r).is_some(), "exercise {} -> {r}", e.id);
        }
    }
    for a in lib.articles() {
        if let Some(t) = &a.troubleshooting {
            for e in &t.simulated_in {
                assert!(
                    lib.exercise(e.as_str()).is_some(),
                    "{} simulated in {e}",
                    a.id
                );
            }
        }
    }
}

#[test]
fn search_finds_fundamentals_vendor_and_troubleshooting_content() {
    let lib = library();
    let first = |text: &str, category: Option<Category>| {
        lib.search(&SearchQuery {
            text: text.into(),
            category,
            limit: 5,
            ..Default::default()
        })
        .first()
        .map(|h| h.id.clone())
        .unwrap_or_default()
    };
    assert_eq!(first("4-20 mA live zero", None), "fundamentals.4-20ma");
    assert_eq!(first("TON on-delay timer", None), "instruction.ton");
    assert!(first("S7-1500", Some(Category::VendorSpecific)).contains("s7-1500"));
    assert_eq!(
        first(
            "stop button normally open block",
            Some(Category::Troubleshooting)
        ),
        "troubleshooting.nc-contact-replaced-by-no"
    );
    assert!(first("Modbus register offset", None).contains("modbus"));
}

#[test]
fn every_practice_exercise_is_solvable_and_the_starter_is_not() {
    let lib = library();
    let mut checked = 0;
    for e in lib.exercises().filter(|e| e.kind == ExerciseKind::Practice) {
        let scene = lib.scene(e.scene.as_str()).unwrap();
        let profile = lib
            .profile(e.platform.as_ref().map_or("generic.iec", |p| p.as_str()))
            .unwrap();
        let station = e
            .starter_station
            .clone()
            .unwrap_or_else(|| scene.reference_station.clone());
        let solution = std::fs::read_to_string(
            root().join(format!("tests/conformance/solutions/{}.st", e.id)),
        )
        .unwrap_or_else(|_| panic!("reference solution for {} is missing", e.id));
        let (passed, failures) =
            score(assess(e, scene, &station, &profile, &solution, &[], None).unwrap());
        assert!(passed, "{}: reference solution fails: {failures}", e.id);
        let (starter_passed, _) =
            score(assess(e, scene, &station, &profile, &e.starter_program, &[], None).unwrap());
        assert!(
            !starter_passed,
            "{}: the starter program already passes",
            e.id
        );
        checked += 1;
    }
    assert!(checked >= 10, "{checked} practice exercises");
    // The minimum practice set.
    for id in [
        "ex.lamp-pushbutton",
        "ex.motor-jog",
        "ex.motor-start-stop",
        "ex.delayed-start",
        "ex.button-counter",
        "ex.tank-filling",
    ] {
        assert!(lib.exercise(id).is_some(), "{id}");
    }
}

#[test]
fn troubleshooting_exercises_fail_until_repaired_and_pass_afterwards() {
    let lib = library();
    let mut checked = 0;
    for e in lib
        .exercises()
        .filter(|e| e.kind == ExerciseKind::Troubleshooting)
    {
        let t = e.troubleshooting.as_ref().unwrap();
        let scene = lib.scene(e.scene.as_str()).unwrap();
        let profile = lib
            .profile(e.platform.as_ref().map_or("generic.iec", |p| p.as_str()))
            .unwrap();
        let broken = e
            .starter_station
            .clone()
            .unwrap_or_else(|| scene.reference_station.clone());
        let running = t
            .loaded_program
            .clone()
            .unwrap_or_else(|| e.starter_program.clone());
        let filter = Some(t.verification.as_slice());
        let (broken_passes, _) =
            score(assess(e, scene, &broken, &profile, &running, &t.faults, filter).unwrap());
        assert!(
            !broken_passes,
            "{}: verification passes with the fault present",
            e.id
        );
        let (repaired, failures) = score(
            assess(
                e,
                scene,
                &scene.reference_station,
                &profile,
                &running,
                &[],
                filter,
            )
            .unwrap(),
        );
        assert!(
            repaired,
            "{}: verification fails after the repair: {failures}",
            e.id
        );
        assert!(
            !t.root_causes.is_empty()
                && t.root_causes
                    .iter()
                    .all(|r| t.diagnosis_options.iter().any(|o| &o.id == r))
        );
        for fault in &t.faults {
            assert!(
                t.repairs.iter().any(|r| r.clears.contains(&fault.id)),
                "{}: no repair clears {}",
                e.id,
                fault.id
            );
        }
        let debrief = t
            .debrief
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no completion debrief", e.id));
        assert!(
            !debrief.root_cause.trim().is_empty()
                && !debrief.evidence.is_empty()
                && debrief.evidence.iter().all(|x| !x.trim().is_empty())
                && !debrief.repair.trim().is_empty()
                && !debrief.reasoning.trim().is_empty(),
            "{}: incomplete debrief",
            e.id
        );
        // Before the diagnosis the lab shows only general concepts; the guide about its
        // fault belongs to the debrief.
        assert!(
            e.knowledge_refs.is_empty(),
            "{}: use learningRefs/debriefRefs",
            e.id
        );
        assert!(
            !t.learning_refs.is_empty(),
            "{}: no learning references",
            e.id
        );
        for r in &t.learning_refs {
            let a = lib
                .article(r.as_str())
                .unwrap_or_else(|| panic!("{} -> {r}", e.id));
            assert_ne!(a.kind, ArticleKind::Troubleshooting, "{} -> {r}", e.id);
            for x in &a.related {
                assert!(
                    !t.debrief_refs.contains(x),
                    "{}: learning reference {r} links to the debrief guide {x}",
                    e.id
                );
            }
        }
        assert!(
            t.debrief_refs.iter().any(|r| lib
                .article(r.as_str())
                .is_some_and(|a| a.kind == ArticleKind::Troubleshooting)),
            "{}: the debrief names no troubleshooting guide",
            e.id
        );
        checked += 1;
    }
    assert!(checked >= 7, "{checked} troubleshooting exercises");
    // Every exercise has a unique position in its curriculum.
    for kind in [ExerciseKind::Practice, ExerciseKind::Troubleshooting] {
        let mut orders: Vec<u16> = lib
            .exercises()
            .filter(|e| e.kind == kind)
            .map(|e| {
                e.curriculum_order
                    .unwrap_or_else(|| panic!("{} has no curriculumOrder", e.id))
            })
            .collect();
        let n = orders.len();
        orders.sort_unstable();
        orders.dedup();
        assert_eq!(orders.len(), n, "{kind:?} curriculum order is not unique");
    }
    // The minimum troubleshooting curriculum: NO/NC, analog range, field supply, drive
    // local/remote, Modbus register map and network addressing.
    for id in [
        "ts.motor-wont-start",
        "ts.part-sensor-inverted",
        "ts.tank-level-reads-high",
        "ts.outputs-dead",
        "ts.vfd-local",
        "ts.remote-level-wrong",
        "ts.remote-level-lost",
    ] {
        assert!(lib.exercise(id).is_some(), "{id}");
    }
}

/// Failed criterion ids of a program in an exercise's isolated assessment.
fn failed(id: &str, program: &str) -> Vec<String> {
    let lib = library();
    let e = lib.exercise(id).unwrap();
    let scene = lib.scene(e.scene.as_str()).unwrap();
    let profile = lib.profile("generic.iec").unwrap();
    let station = scene.reference_station.clone();
    match assess(e, scene, &station, &profile, program, &[], None).unwrap() {
        AssessmentOutcome::Graded(r) => r
            .criteria
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.id.clone())
            .collect(),
        other => panic!("{id}: {other:?}"),
    }
}

#[test]
fn restart_labs_are_graded_by_their_behavior_over_real_restarts() {
    let lib = library();
    // Without VAR RETAIN the count is lost by the warm restart; the cold restart still resets.
    let counter = &lib
        .exercise("ex.retentive-counter")
        .unwrap()
        .starter_program;
    assert_eq!(failed("ex.retentive-counter", counter), ["kept", "lamp"]);
    // The starter resumes the retained fill request after the restart.
    let startup = &lib.exercise("ex.safe-startup").unwrap().starter_program;
    let unsafe_start = failed("ex.safe-startup", startup);
    assert!(
        unsafe_start.contains(&"safe-start".to_string()),
        "{unsafe_start:?}"
    );
    // Another correct fix: the fill request is not retentive, the batch count is.
    let moved = startup.replace(
        "VAR RETAIN\n    Filling : BOOL; (* Fill request. *)\n    Batches",
        "VAR\n    Filling : BOOL; (* Fill request. *)\nEND_VAR\nVAR RETAIN\n    Batches",
    );
    assert_ne!(&moved, startup);
    assert!(failed("ex.safe-startup", &moved).is_empty());
    // Making everything non-retentive starts safe but loses the batch count.
    let nothing_kept = startup.replace("VAR RETAIN", "VAR");
    assert_eq!(failed("ex.safe-startup", &nothing_kept), ["batch-kept"]);
}

#[test]
fn the_function_block_lab_grades_structure_and_behavior() {
    let lib = library();
    let e = lib.exercise("ex.toggle-block").unwrap();
    let scene = lib.scene(e.scene.as_str()).unwrap();
    let profile = lib.profile("generic.iec").unwrap();
    let outcome = |program: &str| {
        assess(
            e,
            scene,
            &scene.reference_station,
            &profile,
            program,
            &[],
            None,
        )
        .unwrap()
    };
    let solution =
        std::fs::read_to_string(root().join("tests/conformance/solutions/ex.toggle-block.st"))
            .unwrap();
    assert!(failed("ex.toggle-block", &solution).is_empty());
    // The starter has the block but not the function: graded as an interface problem.
    match outcome(&e.starter_program) {
        AssessmentOutcome::InterfaceMismatch(p) => assert_eq!(p, ["Write FUNCTION Both."]),
        other => panic!("{other:?}"),
    }
    // Correct behavior written inline, without a function block, is not the exercise.
    let inline = "PROGRAM Main
VAR_INPUT Button1 : BOOL; Button2 : BOOL; END_VAR
VAR_OUTPUT Lamp1 : BOOL; Lamp2 : BOOL; Lamp3 : BOOL; END_VAR
VAR E1 : R_TRIG; E2 : R_TRIG; END_VAR
E1(CLK := Button1); IF E1.Q THEN Lamp1 := NOT Lamp1; END_IF;
E2(CLK := Button2); IF E2.Q THEN Lamp2 := NOT Lamp2; END_IF;
Lamp3 := Lamp1 AND Lamp2;
END_PROGRAM";
    match outcome(inline) {
        AssessmentOutcome::InterfaceMismatch(p) => {
            assert_eq!(p, ["Write FUNCTION_BLOCK Toggle.", "Write FUNCTION Both."])
        }
        other => panic!("{other:?}"),
    }
    // A function block where a function is required, although it behaves correctly.
    let block = solution
        .replace(
            "FUNCTION Both : BOOL\nVAR_INPUT\n    A : BOOL;\n    B : BOOL;\nEND_VAR\nBoth := A AND B;\nEND_FUNCTION",
            "FUNCTION_BLOCK Both\nVAR_INPUT\n    A : BOOL;\n    B : BOOL;\nEND_VAR\nVAR_OUTPUT\n    Q : BOOL;\nEND_VAR\nQ := A AND B;\nEND_FUNCTION_BLOCK",
        )
        .replace(
            "    Toggle2 : Toggle; (* One instance for S2 and H2. *)\n",
            "    Toggle2 : Toggle; (* One instance for S2 and H2. *)\n    Both1 : Both;\n",
        )
        .replace(
            "Lamp3 := Both(Lamp1, Lamp2);",
            "Both1(A := Lamp1, B := Lamp2, Q => Lamp3);",
        );
    assert_eq!(block.matches("Both1").count(), 2, "{block}");
    match outcome(&block) {
        AssessmentOutcome::InterfaceMismatch(p) => assert_eq!(p, ["'Both' must be a FUNCTION."]),
        other => panic!("{other:?}"),
    }
    // Toggling on the level flickers while the button is held.
    let level = solution.replace(
        "Edge(CLK := Press);\nIF Edge.Q THEN",
        "Edge(CLK := Press);\nIF Press THEN",
    );
    assert_ne!(level, solution);
    assert!(failed("ex.toggle-block", &level).contains(&"hold".to_string()));
    // One shared instance drives both lamps.
    let shared = solution.replace(
        "Toggle2(Press := Button2, On => Lamp2);",
        "Toggle1(Press := Button2, On => Lamp2);",
    );
    assert_ne!(shared, solution);
    assert!(failed("ex.toggle-block", &shared).contains(&"independent".to_string()));
    // OR instead of AND.
    let or = solution.replace("Both := A AND B;", "Both := A OR B;");
    assert_ne!(or, solution);
    assert_eq!(failed("ex.toggle-block", &or), ["only-one", "both-off"]);
}

#[test]
fn an_exercise_may_not_require_the_same_pou_twice() {
    let lib = library();
    let mut e = lib.exercise("ex.toggle-block").unwrap().clone();
    let scene = lib.scene(e.scene.as_str()).unwrap();
    lab_model::validate_exercise(&e, scene).unwrap();
    let mut again = e.required_pous[0].clone();
    again.name = lab_model::ids::TagName::new("TOGGLE").unwrap();
    e.required_pous.push(again);
    let error = lab_model::validate_exercise(&e, scene)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("required POU 'TOGGLE' is duplicated"),
        "{error}"
    );
}

#[test]
fn the_pid_lab_grades_offset_disturbance_and_bumpless_transfer() {
    let id = "ex.heater-pid";
    let lib = library();
    let solution =
        std::fs::read_to_string(root().join("tests/conformance/solutions/ex.heater-pid.st"))
            .unwrap();
    assert!(failed(id, &solution).is_empty());
    // P only: it scales and never overshoots, but stays below the setpoint, cannot
    // compensate the open door and has no MAN mode.
    let starter = &lib.exercise(id).unwrap().starter_program;
    assert_eq!(
        failed(id, starter),
        ["reaches", "no-offset", "recovers", "manual", "bumpless"]
    );
    // Switching the output with an IF instead of the block's MANUAL: MAN works, but the
    // controller wound up while it was not in charge, so AUTO starts with a jump.
    let switched = solution.replace(
        "MANUAL := NOT AutoMode, MV_MAN := ManualPower, MV => Power);",
        "MV => Power);\nIF NOT AutoMode THEN\n    Power := ManualPower;\nEND_IF;",
    );
    assert!(switched.contains("Power := ManualPower;"), "{switched}");
    assert_eq!(failed(id, &switched), ["bumpless"]);
    // A very short integral time makes the loop swing around the setpoint.
    let nervous = solution.replace("Ti : TIME := T#20s;", "Ti : TIME := T#2s;");
    assert!(nervous.contains("T#2s"), "{nervous}");
    let swinging = failed(id, &nervous);
    assert!(swinging.contains(&"no-offset".to_string()), "{swinging:?}");
    // Heating past 65 degC is an overshoot, whatever the cause.
    let hot = solution.replace("Setpoint : REAL := 60.0;", "Setpoint : REAL := 70.0;");
    assert!(hot.contains("70.0"), "{hot}");
    let overshoot = failed(id, &hot);
    assert!(
        overshoot.contains(&"no-overshoot".to_string()),
        "{overshoot:?}"
    );
}

#[test]
fn a_ladder_solution_is_graded_like_its_structured_text_equivalent() {
    let id = "ex.motor-start-stop";
    let lad = std::fs::read_to_string(
        root().join("tests/conformance/solutions/ex.motor-start-stop.lad.json"),
    )
    .unwrap();
    assert!(lad_frontend::is_lad(&lad));
    // The ladder program passes every criterion of the exercise, like the ST solution.
    assert!(failed(id, &lad).is_empty(), "{:?}", failed(id, &lad));
    // Without the self-holding branch both languages fail the same criteria.
    let st = std::fs::read_to_string(root().join(format!("tests/conformance/solutions/{id}.st")))
        .unwrap();
    let st_no_hold = st.replace("(StartPB OR MotorContactor)", "StartPB");
    assert_ne!(st_no_hold, st);
    let mut doc: serde_json::Value = serde_json::from_str(&lad).unwrap();
    doc["rungs"][0]["network"]["items"][0] = serde_json::json!({
        "type": "contact", "id": 4, "contact": "normallyOpen", "variable": "StartPB"
    });
    let lad_no_hold = doc.to_string();
    let missing = failed(id, &st_no_hold);
    assert!(!missing.is_empty());
    assert_eq!(failed(id, &lad_no_hold), missing);
}
