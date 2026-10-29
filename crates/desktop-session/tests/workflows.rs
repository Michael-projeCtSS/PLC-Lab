//! End-to-end application workflows over the real knowledge pack: Modbus commissioning and
//! fault finding, the drive troubleshooting lab with its debrief, project persistence, and
//! bounded responses during long runs of large programs.
use app_contract::*;
use desktop_session::{AppInfo, AppPaths, DesktopApp};
use std::path::PathBuf;

const V: u32 = CONTRACT_VERSION;
const MODBUS_SOLUTION: &str =
    include_str!("../../../tests/conformance/solutions/ex.modbus-level.st");

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
    let dir = std::env::temp_dir().join(format!("plclab-flow-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let app = DesktopApp::new(paths(&dir), info());
    Fixture { app, dir }
}
fn paths(dir: &std::path::Path) -> AppPaths {
    AppPaths {
        knowledge_roots: vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../knowledge")],
        projects_dir: dir.join("projects"),
        progress_file: dir.join("progress.json"),
        recent_file: dir.join("recent.json"),
    }
}
fn info() -> AppInfo {
    AppInfo {
        app_version: "test".into(),
        build_context: "test".into(),
    }
}

fn open(app: &DesktopApp, request: ProjectCreateRequest) -> (ProjectDto, WorkspaceDto) {
    let project = app.create_project(request).unwrap();
    let workspace = app
        .open_workspace(WorkspaceOpenRequest {
            contract_version: V,
            project_id: project.id.clone(),
        })
        .unwrap();
    (project, workspace)
}
fn exercise(app: &DesktopApp, id: &str) -> (ProjectDto, WorkspaceDto) {
    open(
        app,
        ProjectCreateRequest {
            contract_version: V,
            name: String::new(),
            exercise: Some(id.into()),
            scene: None,
            platform: None,
        },
    )
}

fn step(app: &DesktopApp, id: ExactU64, n: usize) -> LiveFrameDto {
    let mut frame = None;
    for _ in 0..n {
        frame = Some(
            app.simulation(SimulationRequest {
                contract_version: V,
                workspace_id: id,
                command: "STEP".into(),
            })
            .unwrap()
            .0,
        );
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
fn press(app: &DesktopApp, id: ExactU64, component: &str) {
    operate(app, id, component, OperateActionDto::Press);
    step(app, id, 3);
    operate(app, id, component, OperateActionDto::Release);
}
fn quantity(frame: &LiveFrameDto, component: &str, name: &str) -> SignalValueDto {
    let c = frame.components.iter().find(|c| c.id == component).unwrap();
    c.quantities
        .iter()
        .find(|q| q.name == name)
        .unwrap_or_else(|| panic!("{component}.{name}"))
        .value
}
fn is_true(value: SignalValueDto) -> bool {
    matches!(value, SignalValueDto::Bool { value: true })
}
fn load(app: &DesktopApp, id: ExactU64, source: &str, revision: u64, station: Option<StationDto>) {
    let (result, _) = app
        .build_and_load(BuildLoadRequest {
            contract_version: V,
            workspace_id: id,
            source: source.into(),
            revision: ExactU64(revision),
            station,
            run_after_load: false,
        })
        .unwrap();
    assert_eq!(result.outcome, "LOADED", "{:?}", result.message);
    app.plc_mode(PlcModeRequest {
        contract_version: V,
        workspace_id: id,
        mode: "RUN".into(),
    })
    .unwrap();
}
fn network(app: &DesktopApp, id: ExactU64) -> NetworkDto {
    app.network(WorkspaceRequest {
        contract_version: V,
        workspace_id: id,
    })
    .unwrap()
}
fn ping(app: &DesktopApp, id: ExactU64, ip: &str) -> String {
    app.ping(PingRequest {
        contract_version: V,
        workspace_id: id,
        ip: ip.into(),
    })
    .unwrap()
    .result
}
fn check(
    app: &DesktopApp,
    id: ExactU64,
    source: &str,
    station: Option<StationDto>,
) -> GradeReportDto {
    app.check(CheckRequest {
        contract_version: V,
        workspace_id: id,
        source: source.into(),
        station,
    })
    .unwrap()
}
fn diagnose(app: &DesktopApp, id: ExactU64, option: &str) -> bool {
    app.diagnose(DiagnosisRequest {
        contract_version: V,
        workspace_id: id,
        option: option.into(),
    })
    .unwrap()
    .correct
}
fn repair(app: &DesktopApp, id: ExactU64, repair: &str) {
    app.repair(RepairRequest {
        contract_version: V,
        workspace_id: id,
        repair: repair.into(),
    })
    .unwrap();
}

#[test]
fn modbus_commissioning_success_wrong_register_wrong_address_and_correction() {
    let f = fixture("modbus");
    let (project, w) = exercise(&f.app, "ex.modbus-level");
    let id = w.workspace_id;
    let good = project.station.clone();

    // Success: the scan connects and the PLC reads the transmitter's level register.
    load(&f.app, id, MODBUS_SOLUTION, 1, None);
    let frame = step(&f.app, id, 10);
    let net = network(&f.app, id);
    let scan = &net.scans[0];
    assert_eq!(scan.status, "OK", "{scan:?}");
    assert!(scan.connected && scan.responses.0 > 0 && scan.last_round_trip_us.is_some());
    let SignalValueDto::Count { value: level } = quantity(&frame, "tank", "levelPermille") else {
        panic!("level")
    };
    assert_eq!(
        i64::from(scan.registers[0]),
        level,
        "register 0 is the level in permille"
    );
    assert_eq!(ping(&f.app, id, "192.168.0.20"), "REPLY");

    // Wrong register: the read starts one register late. Communication stays OK, the
    // value is the float's high word.
    let mut shifted = good.clone();
    shifted.network.modbus_tcp[0].start_address = 1;
    load(&f.app, id, MODBUS_SOLUTION, 2, Some(shifted.clone()));
    step(&f.app, id, 10);
    let scan = network(&f.app, id).scans[0].clone();
    assert_eq!(scan.status, "OK");
    assert_ne!(i64::from(scan.registers[0]), level, "{scan:?}");
    // Reading past the transmitter's register map is rejected with exception 02.
    let mut beyond = good.clone();
    beyond.network.modbus_tcp[0].start_address = 2;
    load(&f.app, id, MODBUS_SOLUTION, 3, Some(beyond));
    step(&f.app, id, 10);
    assert_eq!(network(&f.app, id).scans[0].status, "EXCEPTION_02");

    // Wrong address: nobody answers at .21 (timeout); another subnet has no route.
    let mut absent = good.clone();
    absent.network.modbus_tcp[0].server = "192.168.0.21".into();
    load(&f.app, id, MODBUS_SOLUTION, 4, Some(absent));
    step(&f.app, id, 20);
    let scan = network(&f.app, id).scans[0].clone();
    assert_eq!(scan.status, "TIMEOUT", "{scan:?}");
    assert!(scan.timeouts.0 > 0 && !scan.connected);
    assert_eq!(ping(&f.app, id, "192.168.0.21"), "TIMEOUT");
    let mut foreign = good.clone();
    foreign.network.modbus_tcp[0].server = "192.168.1.20".into();
    load(&f.app, id, MODBUS_SOLUTION, 5, Some(foreign));
    step(&f.app, id, 10);
    assert_eq!(network(&f.app, id).scans[0].status, "NO_ROUTE");
    assert_eq!(ping(&f.app, id, "192.168.1.20"), "NO_ROUTE");
    let events = f
        .app
        .events(EventsRequest {
            contract_version: V,
            workspace_id: id,
            after: None,
            limit: 500,
        })
        .unwrap();
    assert!(
        events.events.iter().any(|e| e.kind.contains("NETWORK")),
        "network diagnostics appear in the event log"
    );

    // Correction: downloading the documented configuration restores communication, and
    // grading uses the corrected station.
    load(&f.app, id, MODBUS_SOLUTION, 6, Some(good.clone()));
    step(&f.app, id, 10);
    assert_eq!(network(&f.app, id).scans[0].status, "OK");
    assert!(check(&f.app, id, MODBUS_SOLUTION, Some(good)).passed);
    assert!(!check(&f.app, id, MODBUS_SOLUTION, Some(shifted)).passed);
}

#[test]
fn network_troubleshooting_lab_from_symptom_to_debrief() {
    let f = fixture("net-lab");
    let (project, w) = exercise(&f.app, "ts.remote-level-lost");
    let id = w.workspace_id;
    let source = project.programs[0].source.clone();
    step(&f.app, id, 20);
    let scan = network(&f.app, id).scans[0].clone();
    assert_eq!(scan.status, "TIMEOUT", "{scan:?}");
    assert_eq!(ping(&f.app, id, "192.168.0.20"), "TIMEOUT");
    assert_eq!(ping(&f.app, id, "192.168.1.20"), "NO_ROUTE");
    let report = check(&f.app, id, &source, None);
    assert!(!report.passed && report.debrief.is_none());
    assert!(!diagnose(&f.app, id, "cable"));
    assert!(diagnose(&f.app, id, "lt-address"));
    repair(&f.app, id, "set-address");
    step(&f.app, id, 10);
    assert_eq!(network(&f.app, id).scans[0].status, "OK");
    let report = check(&f.app, id, &source, None);
    assert!(report.passed, "{:?}", report.criteria);
    let debrief = report.debrief.expect("a passed lab shows its debrief");
    assert!(debrief.root_cause.contains("192.168.1.20"));
    assert!(debrief.evidence.len() >= 3);
}

#[test]
fn drive_left_in_local_control_lab_from_symptom_to_debrief() {
    let f = fixture("vfd-lab");
    let (project, w) = exercise(&f.app, "ts.vfd-local");
    let id = w.workspace_id;
    assert_eq!(w.frame.plc.mode, "RUN");
    let source = project.programs[0].source.clone();
    assert!(
        !serde_json::to_string(&w)
            .unwrap()
            .contains("driveLocalControl"),
        "the workspace does not reveal the fault"
    );

    // Symptom: the PLC commands the drive, the drive reports local control and stays still.
    press(&f.app, id, "start");
    let frame = step(&f.app, id, 30);
    assert!(
        is_true(quantity(&frame, "drive", "command")),
        "run terminal ON"
    );
    assert!(!is_true(quantity(&frame, "drive", "running")));
    assert!(!is_true(quantity(&frame, "drive", "remote")));
    let report = check(&f.app, id, &source, None);
    assert!(!report.passed && report.debrief.is_none());

    assert!(!diagnose(&f.app, id, "drive-trip"));
    assert!(diagnose(&f.app, id, "drive-local"));
    repair(&f.app, id, "ack-drive");
    let frame = step(&f.app, id, 5);
    assert!(
        !is_true(quantity(&frame, "drive", "running")),
        "a useless repair changes nothing"
    );
    repair(&f.app, id, "set-remote");
    let frame = step(&f.app, id, 25);
    assert!(is_true(quantity(&frame, "drive", "remote")));
    assert!(is_true(quantity(&frame, "drive", "running")));
    let SignalValueDto::Count { value: speed } = quantity(&frame, "drive", "speedPermille") else {
        panic!("speed")
    };
    assert_eq!(speed, 1000, "the 2 s ramp has finished after 2.5 s");

    let report = check(&f.app, id, &source, None);
    assert!(report.passed, "{:?}", report.criteria);
    let debrief = report.debrief.expect("debrief");
    assert!(debrief.root_cause.contains("Local"));
    assert!(debrief.repair.contains("Remote"));
}

#[test]
fn projects_persist_source_watch_station_and_platform_and_reject_bad_files() {
    let f = fixture("persist");
    let (project, w) = open(
        &f.app,
        ProjectCreateRequest {
            contract_version: V,
            name: "Drive bench".into(),
            exercise: None,
            scene: Some("scene.conveyor-drive".into()),
            platform: Some("siemens.s7-1500".into()),
        },
    );
    assert_eq!(project.platform, "siemens.s7-1500");
    let source = format!(
        "{}\n",
        project.programs[0].source.replace(
            "END_PROGRAM",
            "DriveRun := StartPB AND StopPB;\nEND_PROGRAM"
        )
    );
    let mut edited = project.clone();
    edited.programs[0].source = source.clone();
    edited.watch = vec!["plc.DriveRun".into(), "process.drive.speedPermille".into()];
    edited.station.network.interface = None;
    edited.simulation.speed_percent = 50;
    f.app
        .save_project(ProjectSaveRequest {
            contract_version: V,
            project: edited.clone(),
        })
        .unwrap();
    f.app
        .close_workspace(WorkspaceRequest {
            contract_version: V,
            workspace_id: w.workspace_id,
        })
        .unwrap();

    // A new application instance over the same folders sees the same project.
    let restarted = DesktopApp::new(paths(&f.dir), info());
    let reopened = restarted
        .project(IdRequest {
            contract_version: V,
            id: project.id.clone(),
        })
        .unwrap();
    assert_eq!(reopened.programs[0].source, source);
    assert_eq!(reopened.watch, edited.watch);
    assert_eq!(reopened.station, edited.station);
    assert_eq!(reopened.platform, "siemens.s7-1500");
    assert_eq!(reopened.simulation.speed_percent, 50);
    let w = restarted
        .open_workspace(WorkspaceOpenRequest {
            contract_version: V,
            project_id: project.id.clone(),
        })
        .unwrap();
    assert_eq!(w.platform.id, "siemens.s7-1500");

    // Save As after the restart copies the complete state under a new identity.
    let copy = restarted
        .save_project_as(ProjectSaveAsRequest {
            contract_version: V,
            project_id: project.id.clone(),
            name: "Drive bench copy".into(),
        })
        .unwrap();
    assert_ne!(copy.id, project.id);
    assert_eq!(copy.name, "Drive bench copy");
    assert_eq!(copy.programs[0].source, source);
    assert_eq!(copy.watch, edited.watch);
    assert_eq!(copy.station, edited.station);
    assert_eq!(copy.platform, "siemens.s7-1500");
    assert_eq!(copy.scene, "scene.conveyor-drive");
    let original = restarted
        .project(IdRequest {
            contract_version: V,
            id: project.id.clone(),
        })
        .unwrap();
    assert_eq!(original.name, "Drive bench", "the original is unchanged");

    // Corrupt and future-version files in the projects folder are reported, not fatal.
    let dir = f.dir.join("projects");
    std::fs::write(dir.join("broken.plclab"), "{ this is not json").unwrap();
    let mut future: serde_json::Value = serde_json::from_str(
        &restarted
            .export_project(IdRequest {
                contract_version: V,
                id: project.id.clone(),
            })
            .unwrap()
            .text,
    )
    .unwrap();
    future["formatVersion"] = serde_json::json!(99);
    future["id"] = serde_json::json!("future");
    std::fs::write(dir.join("future.plclab"), future.to_string()).unwrap();
    let list = restarted
        .projects(VersionedRequest {
            contract_version: V,
        })
        .unwrap();
    assert!(list.projects.iter().any(|p| p.id == project.id));
    assert!(
        list.problems.iter().any(|p| p.starts_with("broken:")),
        "{:?}",
        list.problems
    );
    assert!(
        list.problems
            .iter()
            .any(|p| p.starts_with("future:") && p.contains("99")),
        "{:?}",
        list.problems
    );
    let imported = restarted.import_project(ProjectImportRequest {
        contract_version: V,
        text: future.to_string(),
    });
    assert_eq!(imported.unwrap_err().code, "APP-PROJECT-VERSION");
    let mut unknown_scene = future.clone();
    unknown_scene["formatVersion"] = serde_json::json!(1);
    unknown_scene["scene"] = serde_json::json!("scene.does-not-exist");
    let imported = restarted.import_project(ProjectImportRequest {
        contract_version: V,
        text: unknown_scene.to_string(),
    });
    assert!(
        imported.is_err(),
        "a project for a missing scene is rejected"
    );
    let oversized = restarted.import_project(ProjectImportRequest {
        contract_version: V,
        text: " ".repeat(5 * 1024 * 1024),
    });
    assert!(oversized.is_err());
}

#[test]
fn long_runs_of_large_programs_keep_every_response_bounded() {
    let f = fixture("scale");
    let (project, w) = open(
        &f.app,
        ProjectCreateRequest {
            contract_version: V,
            name: "Scale".into(),
            exercise: None,
            scene: Some("scene.io-basics".into()),
            platform: None,
        },
    );
    let id = w.workspace_id;
    let mut decls = String::new();
    let mut body = String::new();
    for i in 0..1000 {
        decls.push_str(&format!("V{i} : INT; "));
        body.push_str(&format!("V{i} := V{i} + {};\n", i % 7 + 1));
    }
    let source = project.programs[0]
        .source
        .replace("END_PROGRAM", "Lamp1 := Button1;\nEND_PROGRAM");
    let source = source.replacen("END_VAR", &format!("END_VAR\nVAR {decls} END_VAR"), 1);
    let source = source.replace("Lamp1 := Button1;", &format!("{body}Lamp1 := Button1;"));
    load(&f.app, id, &source, 1, None);

    let mut max_frame = 0usize;
    let mut frame = None;
    for cycle in 0..3000 {
        if cycle % 500 == 0 {
            operate(&f.app, id, "pb1", OperateActionDto::Press);
        }
        if cycle % 500 == 250 {
            operate(&f.app, id, "pb1", OperateActionDto::Release);
        }
        let f1 = step(&f.app, id, 1);
        max_frame = max_frame.max(serde_json::to_vec(&f1).unwrap().len());
        frame = Some(f1);
    }
    let frame = frame.unwrap();
    assert_eq!(frame.cycle.0, 3000);
    let trace = &frame.trace;
    println!(
        "trace: {} retained, {} dropped cycles, {} of {} bytes; largest frame {max_frame} bytes",
        trace.retained_cycles.0, trace.dropped_cycles.0, trace.used_bytes.0, trace.budget_bytes.0
    );
    assert!(trace.used_bytes.0 <= trace.budget_bytes.0 + 512 * 1024);
    assert_eq!(trace.retained_cycles.0 + trace.dropped_cycles.0, 3000);
    assert!(max_frame < 64 * 1024, "{max_frame} bytes per live frame");

    let page = f
        .app
        .variables(VariablesRequest {
            contract_version: V,
            workspace_id: id,
            filter: None,
            offset: 0,
            limit: 100,
        })
        .unwrap();
    let page_bytes = serde_json::to_vec(&page).unwrap().len();
    println!(
        "variables: {} total, page of {} rows = {page_bytes} bytes",
        page.total,
        page.rows.len()
    );
    assert!(page.total >= 1000 && page.rows.len() <= 100);
    assert!(page_bytes < 64 * 1024);
    let last = f
        .app
        .variables(VariablesRequest {
            contract_version: V,
            workspace_id: id,
            filter: Some("V999".into()),
            offset: 0,
            limit: 100,
        })
        .unwrap();
    assert_eq!(last.rows.len(), 1, "filtered lookup finds one variable");

    let signal = f
        .app
        .variables(VariablesRequest {
            contract_version: V,
            workspace_id: id,
            filter: Some("Lamp1".into()),
            offset: 0,
            limit: 10,
        })
        .unwrap()
        .rows[0]
        .signal;
    let trend = f
        .app
        .trend(TrendRequest {
            contract_version: V,
            workspace_id: id,
            signals: vec![signal],
            from_us: ExactU64(0),
            to_us: ExactU64(u64::MAX),
            max_points: 200,
        })
        .unwrap();
    let trend_bytes = serde_json::to_vec(&trend).unwrap().len();
    println!("trend: {trend_bytes} bytes");
    assert!(trend_bytes < 64 * 1024);

    let events = f
        .app
        .events(EventsRequest {
            contract_version: V,
            workspace_id: id,
            after: None,
            limit: 200,
        })
        .unwrap();
    let event_bytes = serde_json::to_vec(&events).unwrap().len();
    println!(
        "events: {} in page, {event_bytes} bytes",
        events.events.len()
    );
    assert!(events.events.len() <= 200 && event_bytes < 128 * 1024);
}

const LABS: [&str; 23] = [
    "cm.vfd-local",
    "cm.vfd-trip",
    "cm.vfd-enable",
    "cm.vfd-keypad",
    "cm.vfd-no-supply",
    "cm.ai-loop-break",
    "cm.ai-loop-supply",
    "cm.ai-voltage-as-current",
    "cm.ai-zero-twenty",
    "cm.ai-offset",
    "cm.dio-input-supply",
    "cm.dio-guard-contact",
    "cm.dio-swapped-buttons",
    "cm.dio-swapped-outputs",
    "cm.dio-feedback",
    "cm.dio-wrong-slot",
    "ts.motor-wont-start",
    "ts.outputs-dead",
    "ts.part-sensor-inverted",
    "ts.vfd-local",
    "ts.tank-level-reads-high",
    "ts.remote-level-lost",
    "ts.remote-level-wrong",
];

/// Everything the learner sees before the diagnosis: the workspace and the exercise page.
fn visible_before_diagnosis(app: &DesktopApp, w: &WorkspaceDto, id: &str) -> String {
    let page = app
        .exercise(IdRequest {
            contract_version: V,
            id: id.into(),
        })
        .unwrap();
    format!(
        "{}\n{}",
        serde_json::to_string(w).unwrap(),
        serde_json::to_string(&page).unwrap()
    )
}

#[test]
fn every_fault_finding_lab_hides_its_cause_until_verified() {
    let library =
        knowledge_core::KnowledgeLibrary::load(&[
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../knowledge")
        ]);
    let f = fixture("secrecy");
    for id in LABS {
        let lab = library.exercise(id).unwrap_or_else(|| panic!("{id}"));
        let t = lab.troubleshooting.as_ref().unwrap();
        let debrief = t.debrief.as_ref().unwrap();
        // The secrets: guide titles about this fault, the debrief, the repairs that clear
        // a fault and the internal fault types.
        let guides: Vec<String> = t
            .debrief_refs
            .iter()
            .map(|r| library.title_of(r).unwrap())
            .collect();
        assert!(!guides.is_empty(), "{id} has no debrief reference");
        let mut secrets: Vec<String> = guides.clone();
        secrets.push(debrief.root_cause.clone());
        secrets.push(debrief.repair.clone());
        secrets.extend(
            t.repairs
                .iter()
                .filter(|r| !r.clears.is_empty())
                .map(|r| r.label.clone()),
        );
        secrets.extend(t.faults.iter().map(|x| {
            serde_json::to_value(&x.fault).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_owned()
        }));

        let (project, w) = exercise(&f.app, id);
        let visible = visible_before_diagnosis(&f.app, &w, id);
        for secret in &secrets {
            assert!(
                !visible.contains(secret.as_str()),
                "{id} reveals {secret:?}"
            );
        }
        assert!(
            !visible.contains("\"troubleshooting."),
            "{id} links a troubleshooting guide before the diagnosis"
        );
        let brief = w
            .exercise
            .as_ref()
            .unwrap()
            .troubleshooting
            .as_ref()
            .unwrap();
        assert!(!brief.diagnosed && brief.repairs.is_empty(), "{id}");
        assert!(
            !w.exercise.as_ref().unwrap().knowledge_refs.is_empty(),
            "{id} offers general concepts"
        );
        // A repair before the diagnosis is refused by the backend, not only hidden.
        if let Some(r) = t.repairs.first() {
            let refused = f.app.repair(RepairRequest {
                contract_version: V,
                workspace_id: w.workspace_id,
                repair: r.id.to_string(),
            });
            assert_eq!(refused.unwrap_err().code, "APP-INVALID-REQUEST", "{id}");
        }
        let early = check(&f.app, w.workspace_id, &project.programs[0].source, None);
        assert!(!early.passed && early.debrief.is_none(), "{id}");

        // A correct diagnosis unlocks the repairs, and the debrief follows verification.
        let result = f
            .app
            .diagnose(DiagnosisRequest {
                contract_version: V,
                workspace_id: w.workspace_id,
                option: t.root_causes[0].to_string(),
            })
            .unwrap();
        assert!(result.correct, "{id}");
        assert_eq!(result.repairs.len(), t.repairs.len(), "{id}");
        let station = if t.repairs.is_empty() {
            // Configuration labs: download the documented configuration of the scene.
            let documented = f
                .app
                .create_project(ProjectCreateRequest {
                    contract_version: V,
                    name: "Reference".into(),
                    exercise: None,
                    scene: Some(lab.scene.to_string()),
                    platform: None,
                })
                .unwrap();
            Some(documented.station)
        } else {
            for r in t.repairs.iter().filter(|r| !r.clears.is_empty()) {
                repair(&f.app, w.workspace_id, r.id.as_str());
            }
            None
        };
        let report = check(&f.app, w.workspace_id, &project.programs[0].source, station);
        assert!(report.passed, "{id}: {:?}", report.criteria);
        let shown = report.debrief.expect("debrief after verification");
        assert_eq!(shown.root_cause, debrief.root_cause, "{id}");
        assert_eq!(
            shown
                .references
                .iter()
                .map(|r| r.title.clone())
                .collect::<Vec<_>>(),
            guides,
            "{id}"
        );
        // Reopening the lab keeps the unlocked repairs, still without the debrief.
        let reopened = f
            .app
            .open_workspace(WorkspaceOpenRequest {
                contract_version: V,
                project_id: project.id.clone(),
            })
            .unwrap();
        let brief = reopened.exercise.unwrap().troubleshooting.unwrap();
        assert!(brief.diagnosed, "{id}");
        assert_eq!(brief.repairs.len(), t.repairs.len(), "{id}");
        f.app
            .close_workspace(WorkspaceRequest {
                contract_version: V,
                workspace_id: reopened.workspace_id,
            })
            .unwrap();
    }
}

#[test]
fn launch_open_requests_open_store_projects_in_place_and_import_others() {
    let f = fixture("open-request");
    let v = VersionedRequest {
        contract_version: V,
    };
    assert_eq!(f.app.take_open_request(v.clone()).unwrap(), None);
    let (project, w) = open(
        &f.app,
        ProjectCreateRequest {
            contract_version: V,
            name: "Bench".into(),
            exercise: None,
            scene: Some("scene.io-basics".into()),
            platform: None,
        },
    );
    // Opening a workspace records the project in the launcher's recent list.
    let recent = project_model::RecentProjects::load(&f.dir.join("recent.json"));
    assert_eq!(recent.projects[0].id.as_str(), project.id);
    assert_eq!(recent.projects[0].name, "Bench");
    f.app
        .close_workspace(WorkspaceRequest {
            contract_version: V,
            workspace_id: w.workspace_id,
        })
        .unwrap();

    // A file of the projects folder opens in place.
    let stored = f
        .dir
        .join("projects")
        .join(format!("{}.plclab", project.id));
    let r = f.app.request_open(&stored);
    assert_eq!(r.project_id.as_deref(), Some(project.id.as_str()));
    assert_eq!(f.app.take_open_request(v.clone()).unwrap(), Some(r));
    assert_eq!(
        f.app.take_open_request(v.clone()).unwrap(),
        None,
        "taken once"
    );

    // A file elsewhere is validated and imported as a copy; the original is untouched.
    let outside = f.dir.join("elsewhere with spaces");
    std::fs::create_dir_all(&outside).unwrap();
    let exported = f
        .app
        .export_project(IdRequest {
            contract_version: V,
            id: project.id.clone(),
        })
        .unwrap();
    let copy = outside.join("Shared bench.plclab");
    std::fs::write(&copy, &exported.text).unwrap();
    let r = f.app.request_open(&copy);
    let imported = r.project_id.clone().expect("imported");
    assert_ne!(imported, project.id);
    assert_eq!(r.name.as_deref(), Some("Bench"));
    assert_eq!(std::fs::read_to_string(&copy).unwrap(), exported.text);

    // Everything else is refused with an explanation and opens nothing.
    std::fs::write(outside.join("notes.txt"), "{}").unwrap();
    std::fs::write(outside.join("broken.plclab"), "{ not json").unwrap();
    for (path, expected) in [
        (outside.join("notes.txt"), "not a PLC Lab project"),
        (outside.join("missing.plclab"), "not found"),
        (outside.join("broken.plclab"), ""),
        (outside.clone(), "not a PLC Lab project"),
    ] {
        let r = f.app.request_open(&path);
        assert_eq!(r.project_id, None, "{}", path.display());
        let problem = r.problem.expect("a reason");
        assert!(
            problem.to_lowercase().contains(&expected.to_lowercase()),
            "{}: {problem}",
            path.display()
        );
    }
    let projects = f.app.projects(v).unwrap();
    assert_eq!(
        projects.projects.len(),
        2,
        "only the valid copy was imported"
    );
}
