//! Application use-case tests over the real knowledge pack: project lifecycle, build and
//! load, simulation control, causal frames, troubleshooting and isolated grading.
use app_contract::*;
use desktop_session::{AppInfo, AppPaths, DesktopApp, TickOutcome};
use std::path::PathBuf;

const V: u32 = CONTRACT_VERSION;

const MOTOR_SOLUTION: &str = "PROGRAM Main
VAR_INPUT StartPB : BOOL; StopPB : BOOL; MotorFeedback : BOOL; OverloadOK : BOOL; END_VAR
VAR_OUTPUT MotorContactor : BOOL; RunLamp : BOOL; END_VAR
MotorContactor := (StartPB OR MotorContactor) AND StopPB AND OverloadOK;
RunLamp := MotorFeedback;
END_PROGRAM";

struct Fixture {
    app: DesktopApp,
    dir: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn fixture(name: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!("plclab-app-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pack = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../knowledge");
    let app = DesktopApp::new(
        AppPaths {
            knowledge_roots: vec![pack],
            projects_dir: dir.join("projects"),
            progress_file: dir.join("progress.json"),
            recent_file: dir.join("recent.json"),
        },
        AppInfo {
            app_version: "test".into(),
            build_context: "test".into(),
        },
    );
    Fixture { app, dir }
}

fn ws(id: &ExactU64) -> WorkspaceRequest {
    WorkspaceRequest {
        contract_version: V,
        workspace_id: *id,
    }
}

fn open_exercise(app: &DesktopApp, exercise: &str) -> (ProjectDto, WorkspaceDto) {
    let project = app
        .create_project(ProjectCreateRequest {
            contract_version: V,
            name: String::new(),
            exercise: Some(exercise.into()),
            scene: None,
            platform: None,
        })
        .unwrap();
    let workspace = app
        .open_workspace(WorkspaceOpenRequest {
            contract_version: V,
            project_id: project.id.clone(),
        })
        .unwrap();
    (project, workspace)
}

fn signal(w: &WorkspaceDto, signals: &[SignalDto], path: &str) -> u32 {
    signals
        .iter()
        .chain(&w.signals)
        .find(|s| s.path == path)
        .unwrap_or_else(|| panic!("signal {path}"))
        .id
}

fn step(app: &DesktopApp, id: ExactU64, n: usize) -> LiveFrameDto {
    let mut frame = None;
    for _ in 0..n {
        let (f, start) = app
            .simulation(SimulationRequest {
                contract_version: V,
                workspace_id: id,
                command: "STEP".into(),
            })
            .unwrap();
        assert!(start.is_none());
        frame = Some(f);
    }
    frame.unwrap()
}

fn operate(app: &DesktopApp, id: ExactU64, component: &str, action: OperateActionDto) {
    app.operate(OperateRequest {
        contract_version: V,
        workspace_id: id,
        component: component.into(),
        action,
    })
    .unwrap();
}

fn bool_of(frame: &LiveFrameDto, component: &str, quantity: &str) -> bool {
    let c = frame.components.iter().find(|c| c.id == component).unwrap();
    matches!(
        c.quantities
            .iter()
            .find(|q| q.name == quantity)
            .unwrap()
            .value,
        SignalValueDto::Bool { value: true }
    )
}

#[test]
fn knowledge_pack_loads_and_exposes_exercises() {
    let f = fixture("knowledge");
    let status = f
        .app
        .knowledge_status(VersionedRequest {
            contract_version: V,
        })
        .unwrap();
    assert_eq!(status.packs.len(), 1);
    assert!(
        status.packs[0].errors.is_empty(),
        "{:?}",
        status.packs[0].errors
    );
    let library = f
        .app
        .library(VersionedRequest {
            contract_version: V,
        })
        .unwrap();
    assert!(
        library
            .exercises
            .iter()
            .any(|e| e.id == "ex.motor-start-stop")
    );
    assert!(library.platforms.iter().any(|p| p.id == "generic.iec"));
}

#[test]
fn practice_workflow_build_load_run_and_grade() {
    let f = fixture("practice");
    let (project, w) = open_exercise(&f.app, "ex.motor-start-stop");
    let id = w.workspace_id;
    assert_eq!(w.frame.simulation.state, "STOPPED");
    assert_eq!(w.frame.plc.mode, "STOP");

    // A compile error is reported, not loaded.
    let bad = f
        .app
        .build_and_load(BuildLoadRequest {
            contract_version: V,
            workspace_id: id,
            source: "PROGRAM Main x := ; END_PROGRAM".into(),
            revision: ExactU64(1),
            station: None,
            run_after_load: false,
        })
        .unwrap()
        .0;
    assert_eq!(bad.outcome, "COMPILE_FAILED");
    assert!(!bad.diagnostics.is_empty());

    let (loaded, start) = f
        .app
        .build_and_load(BuildLoadRequest {
            contract_version: V,
            workspace_id: id,
            source: MOTOR_SOLUTION.into(),
            revision: ExactU64(2),
            station: None,
            run_after_load: true,
        })
        .unwrap();
    assert_eq!(loaded.outcome, "LOADED", "{:?}", loaded.message);
    assert_eq!(loaded.frame.plc.mode, "RUN");
    assert_eq!(loaded.frame.simulation.state, "RUNNING");
    let start = start.expect("basic mode starts the simulation");
    assert_eq!(start.period_us, 100_000);

    // An older revision arriving late is superseded and changes nothing.
    let late = f
        .app
        .build_and_load(BuildLoadRequest {
            contract_version: V,
            workspace_id: id,
            source: "PROGRAM Old END_PROGRAM".into(),
            revision: ExactU64(1),
            station: None,
            run_after_load: false,
        })
        .unwrap()
        .0;
    assert_eq!(late.outcome, "SUPERSEDED");
    assert_eq!(late.frame.plc.program_name.as_deref(), Some("Main"));

    // The host pacing loop advances one cycle per tick and exits on a stale lease.
    let TickOutcome::Advanced {
        running,
        next_period_us,
        wake,
    } = f.app.tick(start.lease).unwrap()
    else {
        panic!("advanced")
    };
    assert!(running);
    assert_eq!(next_period_us, 100_000);
    assert_eq!(wake.workspace_id, id);
    let paused = f
        .app
        .simulation(SimulationRequest {
            contract_version: V,
            workspace_id: id,
            command: "PAUSE".into(),
        })
        .unwrap()
        .0;
    assert_eq!(paused.simulation.state, "PAUSED");
    assert_eq!(f.app.tick(start.lease).unwrap(), TickOutcome::Stale);

    // Operator presses Start; the causal frame shows what the PLC read and decided.
    let start_pb = signal(&w, &loaded.signals, "plc.StartPB");
    let contactor = signal(&w, &loaded.signals, "plc.MotorContactor");
    operate(&f.app, id, "start", OperateActionDto::Press);
    let frame = step(&f.app, id, 1);
    let ce = frame.cause_effect.as_ref().unwrap();
    assert!(
        ce.saw
            .iter()
            .any(|e| e.signal == start_pb && e.value == SignalValueDto::Bool { value: true })
    );
    assert!(
        ce.decided
            .iter()
            .any(|e| e.signal == contactor && e.value == SignalValueDto::Bool { value: true })
    );
    operate(&f.app, id, "start", OperateActionDto::Release);
    let frame = step(&f.app, id, 20);
    assert!(
        bool_of(&frame, "motor", "running"),
        "seal-in keeps the motor running"
    );

    // PLC STOP is not simulation stop: the world keeps running and outputs drop.
    let stopped = f
        .app
        .plc_mode(PlcModeRequest {
            contract_version: V,
            workspace_id: id,
            mode: "STOP".into(),
        })
        .unwrap();
    assert_eq!(stopped.plc.mode, "STOP");
    let frame = step(&f.app, id, 5);
    assert_eq!(frame.simulation.state, "PAUSED");
    assert!(!bool_of(&frame, "motor", "running"));

    // Stop Simulation resets the plant but keeps program and switch.
    let reset = f
        .app
        .simulation(SimulationRequest {
            contract_version: V,
            workspace_id: id,
            command: "STOP".into(),
        })
        .unwrap()
        .0;
    assert_eq!(reset.simulation.state, "STOPPED");
    assert_eq!(reset.world_time_us, ExactU64(0));
    assert_eq!(reset.plc.program_name.as_deref(), Some("Main"));

    // Projections are bounded and typed.
    let vars = f
        .app
        .variables(VariablesRequest {
            contract_version: V,
            workspace_id: id,
            filter: Some("motor".into()),
            offset: 0,
            limit: 50,
        })
        .unwrap();
    assert!(
        vars.rows
            .iter()
            .all(|r| r.name.to_lowercase().contains("motor"))
    );
    let hw = f.app.hardware(ws(&id)).unwrap();
    assert_eq!(hw.modules.len(), 2);
    assert!(
        f.app
            .events(EventsRequest {
                contract_version: V,
                workspace_id: id,
                after: None,
                limit: 50
            })
            .unwrap()
            .events
            .iter()
            .any(|e| e.kind == "DOWNLOAD")
    );

    // Grading runs isolated from the live session and records progress.
    let report = f
        .app
        .check(CheckRequest {
            contract_version: V,
            workspace_id: id,
            source: MOTOR_SOLUTION.into(),
            station: None,
        })
        .unwrap();
    assert_eq!(report.outcome, "GRADED");
    assert!(report.passed, "{:?}", report.criteria);
    assert_eq!(report.score, report.max_score);
    let missing_seal = MOTOR_SOLUTION.replace("(StartPB OR MotorContactor)", "StartPB");
    let report = f
        .app
        .check(CheckRequest {
            contract_version: V,
            workspace_id: id,
            source: missing_seal,
            station: None,
        })
        .unwrap();
    assert!(!report.passed);
    assert!(
        report
            .criteria
            .iter()
            .any(|c| c.id == "latches" && !c.passed)
    );
    let library = f
        .app
        .library(VersionedRequest {
            contract_version: V,
        })
        .unwrap();
    let progress = library
        .exercises
        .iter()
        .find(|e| e.id == "ex.motor-start-stop")
        .unwrap();
    assert_eq!((progress.attempts, progress.passed), (2, true));

    // The loaded program was persisted into the project.
    let stored = f
        .app
        .project(IdRequest {
            contract_version: V,
            id: project.id.clone(),
        })
        .unwrap();
    assert_eq!(stored.programs[0].source, MOTOR_SOLUTION);
}

#[test]
fn troubleshooting_workflow_requires_evidence_diagnosis_repair_and_verification() {
    let f = fixture("troubleshooting");
    let (project, w) = open_exercise(&f.app, "ts.motor-wont-start");
    let id = w.workspace_id;
    // The installation was running before the fault was reported.
    assert_eq!(w.frame.plc.mode, "RUN");
    let brief = w
        .exercise
        .as_ref()
        .unwrap()
        .troubleshooting
        .as_ref()
        .unwrap();
    assert!(!brief.symptom.is_empty());
    // The root cause is not revealed in the workspace description.
    assert!(
        !serde_json::to_string(&w)
            .unwrap()
            .contains("contactTypeMismatch")
    );

    operate(&f.app, id, "start", OperateActionDto::Press);
    let frame = step(&f.app, id, 3);
    operate(&f.app, id, "start", OperateActionDto::Release);
    assert!(!bool_of(&frame, "motor", "running"), "symptom reproduces");

    // Evidence: the Stop input terminal shows 0 V with the button released.
    let m = f
        .app
        .measure(MeasureRequest {
            contract_version: V,
            workspace_id: id,
            channel: ChannelRefDto {
                rack: 0,
                slot: 1,
                channel: 1,
            },
        })
        .unwrap();
    assert_eq!((m.kind.as_str(), m.value), ("VOLTAGE", 0));
    f.app
        .note_evidence(EvidenceRequest {
            contract_version: V,
            workspace_id: id,
            evidence: "hardware".into(),
        })
        .unwrap();

    // A wrong guess is recorded; the correct one is accepted.
    let wrong = f
        .app
        .diagnose(DiagnosisRequest {
            contract_version: V,
            workspace_id: id,
            option: "start-contact-broken".into(),
        })
        .unwrap();
    assert!(!wrong.correct);
    let right = f
        .app
        .diagnose(DiagnosisRequest {
            contract_version: V,
            workspace_id: id,
            option: "stop-contact-no".into(),
        })
        .unwrap();
    assert!(right.correct);
    assert_eq!(right.attempts, 2);

    // Verification before the repair fails: the fault is still in the installation.
    let source = project.programs[0].source.clone();
    let before = f
        .app
        .check(CheckRequest {
            contract_version: V,
            workspace_id: id,
            source: source.clone(),
            station: None,
        })
        .unwrap();
    assert_eq!(before.outcome, "GRADED");
    assert!(!before.passed);
    assert!(
        before.debrief.is_none(),
        "the debrief is withheld until the lab passes"
    );
    assert!(
        before
            .criteria
            .iter()
            .any(|c| c.id == "repair" && !c.passed)
    );

    // A useless repair changes nothing; the right repair fixes the live plant.
    f.app
        .repair(RepairRequest {
            contract_version: V,
            workspace_id: id,
            repair: "replace-start-button".into(),
        })
        .unwrap();
    f.app
        .repair(RepairRequest {
            contract_version: V,
            workspace_id: id,
            repair: "replace-stop-contact".into(),
        })
        .unwrap();
    operate(&f.app, id, "start", OperateActionDto::Press);
    step(&f.app, id, 3);
    operate(&f.app, id, "start", OperateActionDto::Release);
    let frame = step(&f.app, id, 20);
    assert!(bool_of(&frame, "motor", "running"));

    let after = f
        .app
        .check(CheckRequest {
            contract_version: V,
            workspace_id: id,
            source,
            station: None,
        })
        .unwrap();
    assert!(after.passed, "{:?}", after.criteria);
    let debrief = after
        .debrief
        .as_ref()
        .expect("a passed lab shows its debrief");
    assert!(debrief.root_cause.contains("normally open"));
    assert!(!debrief.evidence.is_empty() && !debrief.reasoning.is_empty());
    assert_eq!(
        after.score,
        15 + 20 + 50,
        "a wrong first diagnosis halves the diagnosis points"
    );

    // The investigation survives closing and reopening the project.
    f.app.close_workspace(ws(&id)).unwrap();
    let reopened = f
        .app
        .open_workspace(WorkspaceOpenRequest {
            contract_version: V,
            project_id: project.id,
        })
        .unwrap();
    let rid = reopened.workspace_id;
    assert_ne!(rid, id);
    assert!(
        matches!(f.app.frame(FrameRequest { contract_version: V, workspace_id: id, watch: vec![] }), Err(e) if e.code == "APP-WORKSPACE-STALE")
    );
    operate(&f.app, rid, "start", OperateActionDto::Press);
    step(&f.app, rid, 3);
    operate(&f.app, rid, "start", OperateActionDto::Release);
    assert!(
        bool_of(&step(&f.app, rid, 20), "motor", "running"),
        "the repair is reapplied on reopen"
    );
}

#[test]
fn free_lab_projects_round_trip_and_reject_bad_input() {
    let f = fixture("freelab");
    let project = f
        .app
        .create_project(ProjectCreateRequest {
            contract_version: V,
            name: "Tank test bench".into(),
            exercise: None,
            scene: Some("scene.tank-filling".into()),
            platform: None,
        })
        .unwrap();
    assert!(project.programs[0].source.contains("VAR_INPUT"));
    let compiled = f
        .app
        .compile(CompileRequest {
            contract_version: V,
            source: project.programs[0].source.clone(),
        })
        .unwrap();
    assert!(
        compiled.success,
        "the generated starter compiles: {:?}",
        compiled.diagnostics
    );

    let copy = f
        .app
        .save_project_as(ProjectSaveAsRequest {
            contract_version: V,
            project_id: project.id.clone(),
            name: "Copy".into(),
        })
        .unwrap();
    assert_ne!(copy.id, project.id);
    let exported = f
        .app
        .export_project(IdRequest {
            contract_version: V,
            id: copy.id.clone(),
        })
        .unwrap();
    assert!(exported.file_name.ends_with(".plclab"));
    let imported = f
        .app
        .import_project(ProjectImportRequest {
            contract_version: V,
            text: exported.text,
        })
        .unwrap();
    assert_eq!(imported.name, "Copy");
    assert_eq!(
        f.app
            .projects(VersionedRequest {
                contract_version: V
            })
            .unwrap()
            .projects
            .len(),
        3
    );

    let mut edited = project.clone();
    edited.name = String::new();
    assert_eq!(
        f.app
            .save_project(ProjectSaveRequest {
                contract_version: V,
                project: edited
            })
            .unwrap_err()
            .code,
        "APP-PROJECT-INVALID"
    );
    assert_eq!(
        f.app
            .import_project(ProjectImportRequest {
                contract_version: V,
                text: "{not json".into()
            })
            .unwrap_err()
            .code,
        "APP-PROJECT-PARSE"
    );
    let w = f
        .app
        .open_workspace(WorkspaceOpenRequest {
            contract_version: V,
            project_id: project.id,
        })
        .unwrap();
    assert_eq!(
        f.app
            .check(CheckRequest {
                contract_version: V,
                workspace_id: w.workspace_id,
                source: String::new(),
                station: None
            })
            .unwrap_err()
            .code,
        "APP-INVALID-REQUEST"
    );
    let step_running = {
        let (_, start) = f
            .app
            .simulation(SimulationRequest {
                contract_version: V,
                workspace_id: w.workspace_id,
                command: "RUN".into(),
            })
            .unwrap();
        assert!(start.is_some());
        f.app.simulation(SimulationRequest {
            contract_version: V,
            workspace_id: w.workspace_id,
            command: "STEP".into(),
        })
    };
    assert_eq!(step_running.unwrap_err().code, "APP-SIMULATION-RUNNING");
}
