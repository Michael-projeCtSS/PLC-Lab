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
