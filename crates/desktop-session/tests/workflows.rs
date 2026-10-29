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
fn the_network_view_shows_requests_responses_and_bounded_traffic_from_the_model() {
    let f = fixture("traffic");
    let (project, w) = exercise(&f.app, "ex.modbus-level");
    let id = w.workspace_id;
    load(&f.app, id, MODBUS_SOLUTION, 1, None);
    step(&f.app, id, 10);
    let net = network(&f.app, id);
    let scan = &net.scans[0];
    let configured = &project.station.network.modbus_tcp[0];
    // The request is the downloaded configuration; values are decoded by the backend.
    assert_eq!(scan.request.start_address, configured.start_address);
    assert_eq!(scan.request.quantity, configured.quantity);
    assert_eq!(scan.request.function_code, 3);
    assert_eq!(scan.request.timeout_ms, configured.timeout_ms);
    assert_eq!(scan.values.len(), configured.values.len());
    assert!(
        scan.values.iter().all(|v| v.value.is_some()),
        "{:?}",
        scan.values
    );
    assert!(scan.last_request_us.is_some() && scan.last_response_us.is_some());
    assert!(net.devices.iter().all(|d| d.same_subnet == Some(true)));
    // Traffic pairs every response with an earlier request of the same transaction.
    let request = net
        .traffic
        .iter()
        .rev()
        .find(|t| t.kind == "request")
        .unwrap();
    assert_eq!(request.address, Some(configured.start_address));
    assert_eq!(request.unit_id, Some(configured.unit_id));
    let responses: Vec<_> = net
        .traffic
        .iter()
        .filter(|t| t.kind == "response")
        .collect();
    assert!(!responses.is_empty());
    for r in responses {
        assert!(
            net.traffic.iter().any(|q| q.kind == "request"
                && q.connection == r.connection
                && q.transaction == r.transaction
                && q.seq.0 < r.seq.0),
            "{r:?}"
        );
        assert!(r.round_trip_us.is_some());
    }
    // A wrong unit on a gateway-style server is not modeled here (the transmitter ignores
    // the unit); an address past the register map answers exception 02.
    let mut beyond = project.station.clone();
    beyond.network.modbus_tcp[0].start_address = 2;
    load(&f.app, id, MODBUS_SOLUTION, 2, Some(beyond));
    step(&f.app, id, 10);
    let net = network(&f.app, id);
    let exception = net
        .traffic
        .iter()
        .rev()
        .find(|t| t.kind == "exception")
        .unwrap();
    assert_eq!(exception.exception_code, Some(2));
    // Long runs keep the traffic bounded.
    step(&f.app, id, 600);
    assert!(network(&f.app, id).traffic.len() <= 60);
}

#[test]
fn each_network_lab_shows_its_own_evidence_in_the_network_view() {
    let f = fixture("network-labs");
    let connection =
        |net: &NetworkDto, id: &str| net.scans.iter().find(|s| s.id == id).unwrap().clone();
    let kinds = |net: &NetworkDto, id: &str| -> Vec<String> {
        net.traffic
            .iter()
            .filter(|t| t.connection == id)
            .map(|t| t.kind.clone())
            .collect()
    };
    for lab in [
        "cm.net-wrong-ip",
        "cm.net-unit-id",
        "cm.net-timeout",
        "cm.net-word-order",
        "cm.net-register",
        "cm.net-device-offline",
    ] {
        let (_, w) = exercise(&f.app, lab);
        let id = w.workspace_id;
        step(&f.app, id, 20);
        let net = network(&f.app, id);
        let (lt, li) = (connection(&net, "lt401"), connection(&net, "li402"));
        match lab {
            "cm.net-wrong-ip" => {
                assert_eq!(lt.server, "192.168.10.22");
                assert!(
                    kinds(&net, "lt401")
                        .iter()
                        .any(|k| k == "arpUnresolved" || k == "timeout")
                );
                assert_eq!(ping(&f.app, id, "192.168.10.21"), "REPLY");
                assert_eq!(li.status, "OK");
            }
            "cm.net-unit-id" => {
                assert!(
                    net.traffic
                        .iter()
                        .any(|t| t.connection == "li402" && t.exception_code == Some(0x0B))
                );
                assert_eq!(lt.status, "OK");
            }
            "cm.net-timeout" => {
                assert!(
                    li.timeouts.0 > 0 && li.exceptions.0 == 0 && li.responses.0 == 0,
                    "{li:?}"
                );
                assert_eq!(ping(&f.app, id, "192.168.10.30"), "REPLY");
            }
            "cm.net-word-order" => {
                assert_eq!((lt.status.as_str(), li.status.as_str()), ("OK", "OK"));
                let decoded = li.values[0].value.clone().unwrap();
                // 20.0 is 41A0 0000h; swapped words make it almost zero: far from the
                // 20 % the tank holds, enough for the deviation alarm.
                let near_truth = decoded
                    .parse::<f32>()
                    .is_ok_and(|v| (v - 20.0).abs() <= 5.0);
                assert!(
                    !near_truth,
                    "the swapped REAL does not match the level: {decoded}"
                );
                assert_eq!(li.values[0].word_order.as_deref(), Some("lowWordFirst"));
            }
            "cm.net-register" => {
                assert_eq!(li.request.start_address, 101);
                assert!(
                    net.traffic
                        .iter()
                        .any(|t| t.connection == "li402" && t.exception_code == Some(0x02))
                );
            }
            _ => {
                assert!(lt.timeouts.0 > 0 && lt.responses.0 == 0, "{lt:?}");
                assert_eq!(ping(&f.app, id, "192.168.10.21"), "TIMEOUT");
                assert_eq!(li.status, "OK");
            }
        }
        f.app
            .close_workspace(WorkspaceRequest {
                contract_version: V,
                workspace_id: id,
            })
            .unwrap();
    }
}

#[test]
fn each_hardware_lab_shows_its_own_evidence_in_the_hardware_view() {
    let library =
        knowledge_core::KnowledgeLibrary::load(&[
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../knowledge")
        ]);
    let f = fixture("hardware-labs");
    let request = |id| WorkspaceRequest {
        contract_version: V,
        workspace_id: id,
    };
    let measure = |id, slot, channel| {
        f.app
            .measure(MeasureRequest {
                contract_version: V,
                workspace_id: id,
                channel: ChannelRefDto {
                    rack: 0,
                    slot,
                    channel,
                },
            })
            .unwrap()
    };
    let categories = |hw: &HardwareDto| -> Vec<(String, Option<u8>)> {
        hw.findings
            .iter()
            .map(|x| (x.category.clone(), x.slot.map(|s| s.slot)))
            .collect()
    };
    let status = |hw: &HardwareDto, slot: u8| -> String {
        hw.modules
            .iter()
            .find(|m| m.position.slot == slot)
            .map_or("ABSENT".into(), |m| m.status.clone())
    };
    let signal = |hw: &HardwareDto, tag: &str| -> SignalMapRowDto {
        hw.signals.iter().find(|s| s.tag == tag).unwrap().clone()
    };
    for lab in [
        "hw.do-field-supply",
        "hw.do-module-power",
        "hw.wrong-module",
        "hw.sensor-channel",
        "hw.ai-mode",
        "hw.wrong-slot",
    ] {
        let (project, w) = exercise(&f.app, lab);
        let id = w.workspace_id;
        press(&f.app, id, "start");
        step(&f.app, id, 30);
        let hw = f.app.hardware(request(id)).unwrap();
        let found = categories(&hw);
        // The mapping always follows the project: symbol, stable channel id, drawn device.
        let closed = signal(&hw, "InletClosed");
        assert_eq!(closed.channel_id.as_deref(), Some("di-r0s2.ch4"), "{lab}");
        assert_eq!(
            closed.drawn_device.as_deref(),
            Some("Inlet valve XV-301"),
            "{lab}"
        );
        match lab {
            "hw.do-field-supply" => {
                assert_eq!(found, [("FIELD_POWER_MISSING".to_string(), Some(3u8))]);
                assert_eq!(status(&hw, 3), "NO_FIELD_POWER");
                // Commanded in the PLC, no voltage at the terminal, the valve stays shut.
                let valve = signal(&hw, "InletValve");
                assert_eq!(valve.output, Some(true), "{valve:?}");
                assert_eq!(
                    valve.device_state.as_deref(),
                    Some("not energized"),
                    "{valve:?}"
                );
                let terminal = measure(id, 3, 0);
                assert_eq!((terminal.kind.as_str(), terminal.value), ("VOLTAGE", 0));
                assert_eq!(
                    closed.device_state.as_deref(),
                    Some("contact closed"),
                    "{closed:?}"
                );
            }
            "hw.do-module-power" => {
                assert_eq!(found, [("MODULE_POWER_MISSING".to_string(), Some(3u8))]);
                assert_eq!(status(&hw, 3), "NO_MODULE_POWER");
                assert_eq!(measure(id, 3, 0).value, 0);
            }
            "hw.wrong-module" => {
                assert_eq!(found, [("WRONG_MODULE_TYPE".to_string(), Some(4u8))]);
                assert_eq!(
                    hw.findings[0].detected.as_deref(),
                    Some("Digital input 8 x 24 V DC (sinking)")
                );
                let level = signal(&hw, "LevelRaw");
                assert_eq!(level.quality.as_deref(), Some("MODULE_FAULT"), "{level:?}");
            }
            "hw.sensor-channel" => {
                // Nothing in the project differs: only testing the inputs finds the wire.
                assert!(found.is_empty(), "{found:?}");
                assert_eq!(
                    closed.device_state.as_deref(),
                    Some("contact closed"),
                    "{closed:?}"
                );
                assert_eq!(
                    closed.value,
                    Some(SignalValueDto::Bool { value: false }),
                    "{closed:?}"
                );
                let di = hw.modules.iter().find(|m| m.position.slot == 2).unwrap();
                let spare = di.channels.iter().find(|c| c.channel == 9).unwrap();
                assert!(spare.tag.is_none() && spare.led, "{spare:?}");
                // The analog output retransmits the level: 20 % of 4-20 mA is 7.2 mA.
                let ao = measure(id, 5, 0);
                assert_eq!(ao.kind, "CURRENT", "{ao:?}");
                assert!((7_100..=7_300).contains(&ao.value), "{ao:?}");
                let out = signal(&hw, "LevelOut");
                assert_eq!(out.channel_id.as_deref(), Some("ao-r0s5.ch0"));
                assert!(out.drawn_device.is_none(), "{out:?}");
            }
            "hw.ai-mode" => {
                assert_eq!(found, [("CHANNEL_MODE_MISMATCH".to_string(), Some(4u8))]);
                assert_eq!(hw.findings[0].channel.map(|c| c.channel), Some(0));
                // A voltage input reads the unsupplied 2-wire loop as a valid 0.
                let level = signal(&hw, "LevelRaw");
                assert_eq!(
                    level.value,
                    Some(SignalValueDto::Int { value: 0 }),
                    "{level:?}"
                );
                assert_eq!(level.quality.as_deref(), Some("GOOD"), "{level:?}");
            }
            _ => {
                assert_eq!(found, [("WRONG_SLOT".to_string(), Some(4u8))]);
                assert_eq!(status(&hw, 4), "MISSING");
                assert_eq!(
                    signal(&hw, "LevelRaw").quality.as_deref(),
                    Some("MODULE_FAULT")
                );
            }
        }
        // Commissioning consumes the same findings instead of its own rules.
        let checklist = f.app.commissioning(request(id)).unwrap();
        let module_level = found.iter().any(|(c, _)| c != "CHANNEL_MODE_MISMATCH");
        let hardware_ok = checklist
            .categories
            .iter()
            .filter(|c| c.id == "power" || c.id == "hardware")
            .flat_map(|c| &c.items)
            .all(|i| i.state == "OK" || i.state == "NOT_APPLICABLE");
        assert_eq!(
            hardware_ok, !module_level,
            "{lab}: {:?}",
            checklist.categories
        );
        let mode_reported =
            checklist.categories.iter().flat_map(|c| &c.items).any(|i| {
                i.state == "FAULT" && i.findings.iter().any(|x| x.contains("ai-r0s4.ch0"))
            });
        assert_eq!(
            mode_reported,
            lab == "hw.ai-mode",
            "{lab}: {:?}",
            checklist.categories
        );
        let evidence = (found, closed.value);

        // The diagnosis unlocks the repair, which changes the authoritative state; a
        // configuration lab is repaired by downloading the documented configuration.
        let setup = library
            .exercise(lab)
            .and_then(|e| e.troubleshooting.as_ref())
            .unwrap();
        assert!(diagnose(&f.app, id, setup.root_causes[0].as_str()), "{lab}");
        if setup.repairs.is_empty() {
            let documented = f
                .app
                .create_project(ProjectCreateRequest {
                    contract_version: V,
                    name: "Reference".into(),
                    exercise: None,
                    scene: Some("scene.hardware-station".into()),
                    platform: None,
                })
                .unwrap();
            load(
                &f.app,
                id,
                &project.programs[0].source,
                1,
                Some(documented.station),
            );
        } else {
            for r in setup.repairs.iter().filter(|r| !r.clears.is_empty()) {
                repair(&f.app, id, r.id.as_str());
            }
        }
        step(&f.app, id, 5);
        let repaired = f.app.hardware(request(id)).unwrap();
        assert!(
            repaired.findings.is_empty(),
            "{lab}: {:?}",
            repaired.findings
        );
        if lab == "hw.sensor-channel" {
            // The rewired switch now reaches the mapped channel.
            assert_eq!(
                signal(&repaired, "InletClosed").value,
                Some(SignalValueDto::Bool { value: true })
            );
        }
        // Reopening the repaired project replays the repair ...
        f.app.close_workspace(request(id)).unwrap();
        let reopened = f
            .app
            .open_workspace(WorkspaceOpenRequest {
                contract_version: V,
                project_id: project.id.clone(),
            })
            .unwrap();
        step(&f.app, reopened.workspace_id, 5);
        let again = f.app.hardware(request(reopened.workspace_id)).unwrap();
        assert!(again.findings.is_empty(), "{lab}: {:?}", again.findings);
        f.app
            .close_workspace(request(reopened.workspace_id))
            .unwrap();
        // ... and a new attempt at the lab starts with the fault and the same evidence.
        let (_, fresh) = exercise(&f.app, lab);
        step(&f.app, fresh.workspace_id, 5);
        let restored = f.app.hardware(request(fresh.workspace_id)).unwrap();
        assert_eq!(
            (
                categories(&restored),
                signal(&restored, "InletClosed").value
            ),
            evidence,
            "{lab}: the fault is back"
        );
        f.app.close_workspace(request(fresh.workspace_id)).unwrap();
    }
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
    edited.watch_tables = vec![WatchTableDto {
        id: "drive".into(),
        name: "Drive".into(),
        rows: vec![
            WatchRowDto {
                signal: "plc.DriveRun".into(),
                format: "auto".into(),
            },
            WatchRowDto {
                signal: "process.drive.speedPermille".into(),
                format: "hex".into(),
            },
        ],
    }];
    edited.station.network.interface = None;
    // A moved module keeps its stable id and takes its mapped channels along, as the
    // station editor does.
    let moved = edited.station.modules[0].clone();
    let free = 9;
    edited.station.modules[0].position.slot = free;
    for mapping in edited.station.io_map.iter_mut() {
        if mapping.channel.rack == moved.position.rack
            && mapping.channel.slot == moved.position.slot
        {
            mapping.channel.slot = free;
        }
    }
    edited.station.modules.sort_by_key(|m| m.position.slot);
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
    assert_eq!(reopened.watch_tables, edited.watch_tables);
    assert_eq!(reopened.station, edited.station);
    let reopened_module = reopened
        .station
        .modules
        .iter()
        .find(|m| m.position.slot == free)
        .unwrap();
    assert_eq!(
        reopened_module.id, moved.id,
        "the module id survives the move"
    );
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
    assert_eq!(copy.watch_tables, edited.watch_tables);
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
    // The file itself is the current format and stores every module's stable id.
    assert_eq!(
        future["formatVersion"],
        serde_json::json!(project_model::FORMAT_VERSION)
    );
    let ids: Vec<&str> = future["station"]["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        edited
            .station
            .modules
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>()
    );
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

#[test]
fn project_plc_and_physical_station_stay_three_layers() {
    let f = fixture("three-layers");
    let (project, w) = open(
        &f.app,
        ProjectCreateRequest {
            contract_version: V,
            name: "Layers".into(),
            exercise: None,
            scene: Some("scene.hardware-station".into()),
            platform: None,
        },
    );
    let id = w.workspace_id;
    let request = || WorkspaceRequest {
        contract_version: V,
        workspace_id: id,
    };
    let channel_signal = |signals: &[SignalDto], path: &str| -> (u32, String) {
        let s = signals.iter().find(|s| s.path == path).unwrap();
        (s.id, s.label.clone())
    };
    let (ao_signal, ao_label) = channel_signal(&w.signals, "io.ao-r0s5.ch0");
    assert_eq!(ao_label, "R0/S5/CH0");
    // The project expects the analog output module in slot 7: saved, not downloaded.
    let mut edited = project.clone();
    let ao = edited
        .station
        .modules
        .iter_mut()
        .find(|m| m.id == "ao-r0s5")
        .unwrap();
    ao.position.slot = 7;
    for mapping in edited.station.io_map.iter_mut() {
        if mapping.channel.slot == 5 {
            mapping.channel.slot = 7;
        }
    }
    edited.station.modules.sort_by_key(|m| m.position.slot);
    let saved = f
        .app
        .save_project(ProjectSaveRequest {
            contract_version: V,
            project: edited.clone(),
        })
        .unwrap();
    assert_eq!(saved.station, edited.station, "the project expects slot 7");
    // The PLC still has the downloaded configuration, which matches the station.
    let hw = f.app.hardware(request()).unwrap();
    assert_eq!(hw.station, project.station);
    assert!(hw.findings.is_empty(), "{:?}", hw.findings);
    // Download: the PLC has the project's configuration; the module is still in slot 5.
    let (loaded, _) = f
        .app
        .build_and_load(BuildLoadRequest {
            contract_version: V,
            workspace_id: id,
            source: project.programs[0].source.clone(),
            revision: ExactU64(1),
            station: Some(saved.station.clone()),
            run_after_load: false,
        })
        .unwrap();
    assert_eq!(loaded.outcome, "LOADED", "{:?}", loaded.message);
    let hw = f.app.hardware(request()).unwrap();
    assert_eq!(hw.station, saved.station);
    let found: Vec<&str> = hw.findings.iter().map(|x| x.category.as_str()).collect();
    assert_eq!(found, ["WRONG_SLOT"]);
    // The canonical channel signal is the same signal; only its location label moved.
    assert_eq!(
        channel_signal(&loaded.signals, "io.ao-r0s5.ch0"),
        (ao_signal, "R0/S7/CH0".to_owned())
    );
}

#[test]
fn saved_and_downloaded_configurations_survive_reopen_and_restart() {
    let f = fixture("saved-downloaded");
    let (project, w) = open(
        &f.app,
        ProjectCreateRequest {
            contract_version: V,
            name: "Saved and downloaded".into(),
            exercise: None,
            scene: Some("scene.hardware-station".into()),
            platform: None,
        },
    );
    let reopen = |app: &DesktopApp| {
        let w = app
            .open_workspace(WorkspaceOpenRequest {
                contract_version: V,
                project_id: project.id.clone(),
            })
            .unwrap();
        let request = WorkspaceRequest {
            contract_version: V,
            workspace_id: w.workspace_id,
        };
        let hw = app.hardware(request.clone()).unwrap();
        app.close_workspace(request).unwrap();
        let saved = app
            .project(IdRequest {
                contract_version: V,
                id: project.id.clone(),
            })
            .unwrap();
        (saved.station, hw.station)
    };
    // Save an edit without downloading it: the analog output module moves to slot 7.
    let mut edited = project.clone();
    let ao = edited
        .station
        .modules
        .iter_mut()
        .find(|m| m.id == "ao-r0s5")
        .unwrap();
    ao.position.slot = 7;
    for mapping in edited.station.io_map.iter_mut() {
        if mapping.channel.slot == 5 {
            mapping.channel.slot = 7;
        }
    }
    edited.station.modules.sort_by_key(|m| m.position.slot);
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
    // Reopened: the project expects slot 7, the PLC still has what was downloaded.
    let (expected, plc) = reopen(&f.app);
    assert_eq!(expected, edited.station);
    assert_eq!(
        plc, project.station,
        "a saved edit is not downloaded by reopening"
    );
    // Download it, then reopen in a restarted application: the PLC has the new one.
    let w = f
        .app
        .open_workspace(WorkspaceOpenRequest {
            contract_version: V,
            project_id: project.id.clone(),
        })
        .unwrap();
    load(
        &f.app,
        w.workspace_id,
        &project.programs[0].source,
        1,
        Some(edited.station.clone()),
    );
    f.app
        .close_workspace(WorkspaceRequest {
            contract_version: V,
            workspace_id: w.workspace_id,
        })
        .unwrap();
    let restarted = DesktopApp::new(paths(&f.dir), info());
    let (expected, plc) = reopen(&restarted);
    assert_eq!((expected, plc), (edited.station.clone(), edited.station));
    let file = restarted
        .export_project(IdRequest {
            contract_version: V,
            id: project.id.clone(),
        })
        .unwrap()
        .text;
    assert!(file.contains("\"downloadedStation\""), "{file}");
}

#[test]
fn watch_rows_come_from_the_runtime_for_every_kind_of_canonical_signal() {
    let f = fixture("watch-rows");
    let (project, w) = open(
        &f.app,
        ProjectCreateRequest {
            contract_version: V,
            name: "Watch".into(),
            exercise: None,
            scene: Some("scene.hardware-station".into()),
            platform: None,
        },
    );
    let id = w.workspace_id;
    // PLC tag signals exist once a program is loaded.
    let (loaded, _) = f
        .app
        .build_and_load(BuildLoadRequest {
            contract_version: V,
            workspace_id: id,
            source: project.programs[0].source.clone(),
            revision: ExactU64(1),
            station: None,
            run_after_load: false,
        })
        .unwrap();
    assert_eq!(loaded.outcome, "LOADED", "{:?}", loaded.message);
    f.app
        .plc_mode(PlcModeRequest {
            contract_version: V,
            workspace_id: id,
            mode: "RUN".into(),
        })
        .unwrap();
    step(&f.app, id, 10);
    let signal = |signals: &[SignalDto], path: &str| -> u32 {
        signals.iter().find(|s| s.path == path).unwrap().id
    };
    let paths = [
        "plc.LevelRaw",
        "plc.InletValve",
        "io.ai-r0s4.ch0",
        "process.tank.levelPermille",
    ];
    let mut ids: Vec<u32> = paths.iter().map(|p| signal(&loaded.signals, p)).collect();
    f.app
        .force(ForceRequest {
            contract_version: V,
            workspace_id: id,
            signal: ids[1],
            value: Some(SignalValueDto::Bool { value: true }),
        })
        .unwrap();
    // An id the runtime does not know is left out.
    ids.push(u32::MAX);
    let watch = |ids: Vec<u32>| {
        f.app.watch(WatchRequest {
            contract_version: V,
            workspace_id: id,
            signals: ids,
        })
    };
    let live = watch(ids.clone()).unwrap();
    let rows: Vec<&str> = live.rows.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(rows, paths);
    let [level, valve, channel, tank] = &live.rows[..] else {
        panic!("{:?}", live.rows)
    };
    assert_eq!(
        (level.kind.as_str(), level.value_type.as_str()),
        ("PLC_INPUT", "INT")
    );
    assert!(
        level.scan_value.is_some() && level.source.is_some(),
        "{level:?}"
    );
    assert_eq!(level.quality.as_deref(), Some("GOOD"));
    assert_eq!(valve.kind, "PLC_OUTPUT");
    assert_eq!(valve.forced, Some(SignalValueDto::Bool { value: true }));
    assert!(valve.committed.is_some(), "{valve:?}");
    assert_eq!(channel.kind, "CHANNEL");
    assert_eq!(channel.label, "R0/S4/CH0");
    assert_eq!(channel.quality.as_deref(), Some("GOOD"), "{channel:?}");
    assert!(
        channel.source.is_some() && channel.value.is_some(),
        "{channel:?}"
    );
    assert_eq!(tank.kind, "PHYSICAL");
    assert!(
        matches!(tank.value, Some(SignalValueDto::Count { .. })),
        "{tank:?}"
    );
    assert!(live.cycle.0 >= 10, "{}", live.cycle.0);
    // A watch table holds at most 64 signals.
    let refused = watch(vec![ids[0]; 65]).unwrap_err();
    assert_eq!(refused.code, "APP-INVALID-REQUEST");
    // A moved module keeps its canonical channel: the row still resolves.
    let mut station = project.station.clone();
    let ai = station
        .modules
        .iter_mut()
        .find(|m| m.id == "ai-r0s4")
        .unwrap();
    ai.position.slot = 6;
    for mapping in station.io_map.iter_mut() {
        if mapping.channel.slot == 4 {
            mapping.channel.slot = 6;
        }
    }
    station.modules.sort_by_key(|m| m.position.slot);
    load(&f.app, id, &project.programs[0].source, 2, Some(station));
    let moved = watch(vec![ids[2]]).unwrap();
    assert_eq!(moved.rows[0].path, "io.ai-r0s4.ch0");
    assert_eq!(moved.rows[0].label, "R0/S6/CH0");
}

#[test]
fn forces_are_listed_removed_and_end_with_the_session_or_a_memory_reset() {
    let f = fixture("forces");
    let (project, w) = open(
        &f.app,
        ProjectCreateRequest {
            contract_version: V,
            name: "Forces".into(),
            exercise: None,
            scene: Some("scene.hardware-station".into()),
            platform: None,
        },
    );
    let id = w.workspace_id;
    let (loaded, _) = f
        .app
        .build_and_load(BuildLoadRequest {
            contract_version: V,
            workspace_id: id,
            source: project.programs[0].source.clone(),
            revision: ExactU64(1),
            station: None,
            run_after_load: false,
        })
        .unwrap();
    assert_eq!(loaded.outcome, "LOADED", "{:?}", loaded.message);
    let signal = |path: &str| {
        let found = loaded.signals.iter().find(|s| s.path == path);
        found.unwrap().id
    };
    let request = || WorkspaceRequest {
        contract_version: V,
        workspace_id: id,
    };
    let force = |signal: u32, value: Option<SignalValueDto>| {
        f.app.force(ForceRequest {
            contract_version: V,
            workspace_id: id,
            signal,
            value,
        })
    };
    let (raw, valve) = (signal("plc.LevelRaw"), signal("plc.InletValve"));
    let on = Some(SignalValueDto::Bool { value: true });
    let off = Some(SignalValueDto::Bool { value: false });
    let level = Some(SignalValueDto::Int { value: 1234 });
    force(raw, level).unwrap();
    force(valve, on).unwrap();
    // The tag's type decides what a force may hold.
    assert!(force(raw, on).is_err());
    let listed = f.app.forces(request()).unwrap().forces;
    let targets: Vec<(&str, &str)> = listed
        .iter()
        .map(|x| (x.path.as_str(), x.target.as_str()))
        .collect();
    let input = ("plc.LevelRaw", "INPUT_IMAGE");
    let output = ("plc.InletValve", "OUTPUT_IMAGE");
    assert!(
        targets.contains(&input) && targets.contains(&output),
        "{targets:?}"
    );
    let frame = f.app.remove_all_forces(request()).unwrap();
    assert_eq!(frame.plc.forces, 0);
    let remaining = f.app.forces(request()).unwrap().forces;
    assert!(remaining.is_empty());
    // A memory reset removes forces too.
    force(valve, off).unwrap();
    let reset = f.app.plc_reset(request()).unwrap();
    assert_eq!(reset.plc.forces, 0);
    // Forces belong to the session: a reopened workspace starts without any.
    force(valve, on).unwrap();
    f.app.close_workspace(request()).unwrap();
    let reopened = f
        .app
        .open_workspace(WorkspaceOpenRequest {
            contract_version: V,
            project_id: project.id.clone(),
        })
        .unwrap();
    assert_eq!(reopened.frame.plc.forces, 0);
}

#[test]
fn the_diagnostic_buffer_classifies_pairs_filters_and_pages_its_entries() {
    let library =
        knowledge_core::KnowledgeLibrary::load(&[
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../knowledge")
        ]);
    let f = fixture("diagnostic-buffer");
    let (_, w) = exercise(&f.app, "hw.do-field-supply");
    let id = w.workspace_id;
    step(&f.app, id, 5);
    let page = |severity: Option<&str>, category: Option<&str>, module: Option<&str>| {
        f.app
            .diagnostic_buffer(DiagnosticBufferRequest {
                contract_version: V,
                workspace_id: id,
                offset: 0,
                limit: 50,
                severity: severity.map(str::to_owned),
                category: category.map(str::to_owned),
                module_id: module.map(str::to_owned),
            })
            .unwrap()
    };
    // The missing field supply is an active hardware error of module do-r0s3.
    let errors = page(Some("error"), Some("HARDWARE"), Some("do-r0s3"));
    let supply = errors
        .entries
        .iter()
        .find(|e| e.kind == "NoFieldPower")
        .unwrap();
    assert!(supply.active && supply.cleared_us.is_none(), "{supply:?}");
    assert_eq!(supply.event, "INCOMING");
    assert!(supply.details.iter().any(|d| d.value == "do-r0s3"));
    // The installed program was downloaded and started: CPU events with details.
    let cpu = page(None, Some("CPU"), None);
    assert!(cpu.entries.iter().all(|e| e.category == "CPU"));
    assert!(cpu.entries.iter().any(|e| e.kind == "ProgramDownloaded"));
    let started = cpu
        .entries
        .iter()
        .find(|e| e.kind == "ModeChange")
        .unwrap();
    assert_eq!(
        (started.event.as_str(), started.severity.as_str()),
        ("EVENT", "info")
    );
    let to: Vec<&str> = started
        .details
        .iter()
        .filter(|d| d.name == "to")
        .map(|d| d.value.as_str())
        .collect();
    assert_eq!(to, ["RUN"]);
    // After the repair the condition is cleared, with its clearing time.
    let setup = library
        .exercise("hw.do-field-supply")
        .and_then(|e| e.troubleshooting.as_ref())
        .unwrap();
    assert!(diagnose(&f.app, id, setup.root_causes[0].as_str()));
    let fix = setup.repairs.iter().find(|r| !r.clears.is_empty());
    repair(&f.app, id, fix.unwrap().id.as_str());
    step(&f.app, id, 2);
    let all = page(None, None, Some("do-r0s3"));
    let raised = all
        .entries
        .iter()
        .find(|e| e.kind == "NoFieldPower" && e.event == "INCOMING")
        .unwrap();
    assert!(!raised.active && raised.cleared_us.is_some(), "{raised:?}");
    let mut outgoing = all.entries.iter().filter(|e| e.event == "OUTGOING");
    assert!(outgoing.any(|e| e.kind == "NoFieldPower"));
    // Newest first, bounded pages.
    let everything = page(None, None, None);
    let times: Vec<u64> = everything.entries.iter().map(|e| e.time_us.0).collect();
    assert!(times.windows(2).all(|t| t[0] >= t[1]), "{times:?}");
    assert!(everything.matching as usize >= everything.entries.len());
    assert!(everything.capacity >= 200);
}

#[test]
fn a_large_station_keeps_the_hardware_view_bounded() {
    let f = fixture("large-station");
    let (project, w) = open(
        &f.app,
        ProjectCreateRequest {
            contract_version: V,
            name: "Large station".into(),
            exercise: None,
            scene: Some("scene.hardware-station".into()),
            platform: None,
        },
    );
    let id = w.workspace_id;
    // 1 CPU, 8 DI16, 8 DO16, 4 AI4 and 4 AO2: the scene's modules plus configured
    // extensions that are not installed (the worst case for the comparison).
    let mut station = project.station.clone();
    let mut slot = 6u8;
    for (module, extra, bytes, base) in [
        ("di16.24vdc.sink", 7, 2u16, 2u16),
        ("do16.24vdc", 7, 2, 2),
        ("ai4.ui", 3, 8, 72),
        ("ao2.ui", 3, 4, 68),
    ] {
        for n in 0..extra {
            station.modules.push(ConfiguredModuleDto {
                id: format!("ext-s{slot}"),
                position: SlotRefDto { rack: 0, slot },
                module: module.into(),
                start_address: base + n * bytes,
                channels: Vec::new(),
            });
            slot += 1;
        }
    }
    load(&f.app, id, &project.programs[0].source, 1, Some(station));
    let request = || WorkspaceRequest {
        contract_version: V,
        workspace_id: id,
    };
    let started = std::time::Instant::now();
    let mut largest = 0usize;
    for _ in 0..50 {
        step(&f.app, id, 10);
        let hw = f.app.hardware(request()).unwrap();
        largest = largest.max(serde_json::to_vec(&hw).unwrap().len());
    }
    let hw = f.app.hardware(request()).unwrap();
    let checklist = f.app.commissioning(request()).unwrap();
    let elapsed = started.elapsed();
    let checklist_bytes = serde_json::to_vec(&checklist).unwrap().len();
    println!(
        "large station: 500 scans and 51 hardware views in {elapsed:?}; largest view {largest} bytes, checklist {checklist_bytes} bytes"
    );
    let configured = hw.modules.iter().filter(|m| m.configured.is_some()).count();
    assert_eq!(configured, 25, "power supply and 24 I/O modules");
    assert_eq!(
        hw.modules.iter().map(|m| m.channels.len()).sum::<usize>(),
        8 * 16 + 8 * 16 + 4 * 4 + 4 * 2
    );
    // Every extension is reported once as missing; the installed modules match.
    assert_eq!(hw.findings.len(), 20, "{:?}", hw.findings);
    assert!(hw.findings.iter().all(|x| x.category == "MODULE_MISSING"));
    assert!(largest < 512 * 1024, "{largest} bytes per hardware view");
    assert!(checklist_bytes < 128 * 1024, "{checklist_bytes} bytes");
    assert!(elapsed.as_secs() < 60, "{elapsed:?}");
}

/// Every fault-finding lab of the pack: the secrecy rules apply to all of them, including
/// labs added later.
fn fault_finding_labs(library: &knowledge_core::KnowledgeLibrary) -> Vec<String> {
    let labs: Vec<String> = library
        .exercises()
        .filter(|e| e.troubleshooting.is_some())
        .map(|e| e.id.to_string())
        .collect();
    // Digital, analog, drive, network, hardware and classic troubleshooting labs.
    assert!(labs.len() >= 36, "only {} fault-finding labs", labs.len());
    for family in ["cm.dio-", "cm.ai-", "cm.vfd-", "cm.net-", "hw.", "ts."] {
        assert!(labs.iter().any(|l| l.starts_with(family)), "{family}");
    }
    labs
}

/// Everything the learner sees before the diagnosis: the workspace, the exercise page, and
/// the Hardware and Commissioning views after the station has run.
fn visible_before_diagnosis(app: &DesktopApp, w: &WorkspaceDto, id: &str) -> String {
    let page = app
        .exercise(IdRequest {
            contract_version: V,
            id: id.into(),
        })
        .unwrap();
    let request = || WorkspaceRequest {
        contract_version: V,
        workspace_id: w.workspace_id,
    };
    step(app, w.workspace_id, 20);
    let hardware = app.hardware(request()).unwrap();
    let commissioning = app.commissioning(request()).unwrap();
    format!(
        "{}\n{}\n{}\n{}",
        serde_json::to_string(w).unwrap(),
        serde_json::to_string(&page).unwrap(),
        serde_json::to_string(&hardware).unwrap(),
        serde_json::to_string(&commissioning).unwrap()
    )
}

#[test]
fn every_fault_finding_lab_hides_its_cause_until_verified() {
    let library =
        knowledge_core::KnowledgeLibrary::load(&[
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../knowledge")
        ]);
    let f = fixture("secrecy");
    for id in fault_finding_labs(&library) {
        let id = id.as_str();
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
