use app_contract::*;
use desktop_session::RuntimeSessionService;
use plc_types::PlcReal;
const EXAMPLE: &str = include_str!("../../../scenes/tank-filling/example.st");
fn target(id: RuntimeSessionId) -> RuntimeSessionRequest {
    RuntimeSessionRequest {
        contract_version: CONTRACT_VERSION,
        session_id: id,
    }
}
fn load(service: &RuntimeSessionService, source: &str) -> RuntimeSnapshotDto {
    let LoadProgramResponse::Loaded { snapshot } = service
        .load(LoadStProgramRequest {
            contract_version: CONTRACT_VERSION,
            source: source.into(),
        })
        .unwrap()
    else {
        panic!("Valid real ST required")
    };
    *snapshot
}
fn attach(service: &RuntimeSessionService, id: RuntimeSessionId) -> RuntimeSnapshotDto {
    service
        .attach_process(AttachProcessSceneRequest {
            contract_version: CONTRACT_VERSION,
            session_id: id,
            scene_kind: ProcessSceneKind::TankFilling,
        })
        .unwrap()
}
fn tank(s: &RuntimeSnapshotDto) -> &TankProcessSnapshotDto {
    let Some(ProcessSnapshotDto::TankFilling(tank)) = &s.process else {
        panic!("Tank must be attached")
    };
    tank
}
fn variable<'a>(s: &'a RuntimeSnapshotDto, name: &str) -> &'a RuntimeVariable {
    s.variables
        .iter()
        .find(|v| v.name.eq_ignore_ascii_case(name))
        .unwrap()
}
fn input(
    service: &RuntimeSessionService,
    s: &RuntimeSnapshotDto,
    name: &str,
    value: RuntimeValue,
) -> RuntimeSnapshotDto {
    service
        .set_input(SetRuntimeInputRequest {
            contract_version: CONTRACT_VERSION,
            session_id: s.session_id,
            variable_id: variable(s, name).variable_id,
            value,
        })
        .unwrap()
}
fn bool_value(value: bool) -> RuntimeValue {
    RuntimeValue::Bool { value }
}
fn writable(s: &RuntimeSnapshotDto) -> Vec<RuntimeVariable> {
    s.variables
        .iter()
        .filter(|v| v.role != RuntimeRole::Input)
        .cloned()
        .collect()
}
fn schema(edit: Option<(&str, &str, &str)>) -> String {
    let mut source = String::from("PROGRAM Main\n");
    for (name, kind, role) in [
        ("LevelLow", "BOOL", "VAR_INPUT"),
        ("LevelHigh", "BOOL", "VAR_INPUT"),
        ("LevelPercent", "REAL", "VAR_INPUT"),
        ("InletValve", "BOOL", "VAR_OUTPUT"),
        ("DrainPump", "BOOL", "VAR_OUTPUT"),
    ] {
        let (kind, role) = match edit {
            Some((selected, kind, role)) if selected == name => (kind, role),
            _ => (kind, role),
        };
        if kind.is_empty() {
            continue;
        }
        source.push_str(&format!("{role} {name} : {kind}; END_VAR\n"));
    }
    source.push_str("END_PROGRAM");
    source
}
#[test]
fn explicit_compatible_attachment_stages_initial_sensors_and_owns_only_sensor_inputs() {
    let service = RuntimeSessionService::default();
    let initial = load(&service, EXAMPLE);
    assert!(initial.process.is_none());
    let s = attach(&service, initial.session_id);
    assert_eq!(tank(&s).level_permille, 0);
    assert!(tank(&s).low_level);
    assert!(!tank(&s).high_level);
    assert!(!tank(&s).inlet_valve_open);
    assert!(!tank(&s).drain_pump_running);
    assert_eq!(variable(&s, "LevelLow").value, bool_value(true));
    assert_eq!(variable(&s, "LevelHigh").value, bool_value(false));
    assert_eq!(
        variable(&s, "LevelPercent").value,
        RuntimeValue::Real {
            value: PlcReal::new(0.0).unwrap()
        }
    );
    for v in &s.variables {
        assert_eq!(
            v.input_authority,
            if v.role == RuntimeRole::Input {
                Some(InputAuthority::Process)
            } else {
                None
            }
        );
    }
    assert_eq!(s.completed_scans, initial.completed_scans);
    assert_eq!(s.simulation_time_micros, initial.simulation_time_micros);
}
#[test]
fn actual_tank_source_proves_output_process_sensor_next_scan_feedback_delay() {
    let service = RuntimeSessionService::default();
    let initial = load(&service, EXAMPLE);
    let mut s = attach(&service, initial.session_id);
    for n in 1..=9_u64 {
        s = service.single_scan(target(s.session_id)).unwrap();
        let level = if n == 9 { 800 } else { n as u16 * 100 };
        assert_eq!(tank(&s).level_permille, level);
        assert_eq!(tank(&s).low_level, level <= 200);
        assert_eq!(tank(&s).high_level, n >= 8);
        assert_eq!(tank(&s).inlet_valve_open, n < 9);
        assert_eq!(variable(&s, "InletValve").value, bool_value(n < 9));
        assert_eq!(variable(&s, "LevelHigh").value, bool_value(n >= 8));
        assert_eq!(
            variable(&s, "LevelPercent").value,
            RuntimeValue::Real {
                value: PlcReal::new(f32::from(level) / 10.0).unwrap()
            }
        );
        assert_eq!(s.completed_scans.get(), n);
        assert_eq!(s.simulation_time_micros.as_micros(), n * 100_000);
        assert_eq!(service.snapshot(target(s.session_id)).unwrap(), s);
    }
}
#[test]
fn every_required_binding_rejects_missing_wrong_type_and_wrong_role_without_mutation() {
    for (name, kind, role) in [
        ("LevelLow", "BOOL", "VAR_INPUT"),
        ("LevelHigh", "BOOL", "VAR_INPUT"),
        ("LevelPercent", "REAL", "VAR_INPUT"),
        ("InletValve", "BOOL", "VAR_OUTPUT"),
        ("DrainPump", "BOOL", "VAR_OUTPUT"),
    ] {
        for (replacement, other_role, code) in [
            ("", role, "PLC-PROCESS-001"),
            (
                if kind == "REAL" { "INT" } else { "REAL" },
                role,
                "PLC-PROCESS-002",
            ),
            (kind, "VAR", "PLC-PROCESS-003"),
        ] {
            let service = RuntimeSessionService::default();
            let before = load(&service, &schema(Some((name, replacement, other_role))));
            let error = service
                .attach_process(AttachProcessSceneRequest {
                    contract_version: CONTRACT_VERSION,
                    session_id: before.session_id,
                    scene_kind: ProcessSceneKind::TankFilling,
                })
                .unwrap_err();
            assert_eq!(error.code, code, "{name}");
            assert!(error.message.contains(name));
            assert_eq!(service.snapshot(target(before.session_id)).unwrap(), before);
        }
    }
}
#[test]
fn names_are_case_insensitive_and_extra_inputs_remain_user_owned_with_real_runtime_ids() {
    let source = EXAMPLE
        .replace("LevelLow", "lEvElLoW")
        .replace("LevelHigh", "LEVELHIGH")
        .replace("LevelPercent", "levelpercent")
        .replace("VAR_INPUT", "VAR_INPUT Extra : BOOL;");
    let service = RuntimeSessionService::default();
    let loaded = load(&service, &source);
    let s = attach(&service, loaded.session_id);
    assert_ne!(variable(&s, "LevelLow").variable_id.get(), 1);
    assert_eq!(variable(&s, "LevelLow").name, "lEvElLoW");
    assert_eq!(
        variable(&s, "Extra").input_authority,
        Some(InputAuthority::User)
    );
    let next = input(&service, &s, "Extra", bool_value(true));
    assert_eq!(variable(&next, "Extra").value, bool_value(true));
    assert_eq!(next.process, s.process);
}
#[test]
fn owned_input_writes_are_rejected_in_backend_and_detach_restores_manual_authority() {
    let service = RuntimeSessionService::default();
    let loaded = load(&service, EXAMPLE);
    let s = attach(&service, loaded.session_id);
    for name in ["LevelLow", "LevelHigh", "LevelPercent"] {
        let error = service
            .set_input(SetRuntimeInputRequest {
                contract_version: CONTRACT_VERSION,
                session_id: s.session_id,
                variable_id: variable(&s, name).variable_id,
                value: variable(&s, name).value,
            })
            .unwrap_err();
        assert_eq!(error.code, "PLC-PROCESS-004");
        assert_eq!(service.snapshot(target(s.session_id)).unwrap(), s);
    }
    let detached = service.detach_process(target(s.session_id)).unwrap();
    assert!(detached.process.is_none());
    assert_eq!(detached.completed_scans, s.completed_scans);
    assert_eq!(detached.simulation_time_micros, s.simulation_time_micros);
    assert_eq!(writable(&detached), writable(&s));
    for v in detached
        .variables
        .iter()
        .filter(|v| v.role == RuntimeRole::Input)
    {
        assert_eq!(v.input_authority, Some(InputAuthority::User));
    }
    assert_eq!(
        variable(&detached, "LevelLow").value,
        variable(&s, "LevelLow").value
    );
    let manual = input(&service, &detached, "LevelHigh", bool_value(true));
    let after = service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(variable(&manual, "LevelHigh").value, bool_value(true));
    assert_eq!(variable(&after, "InletValve").value, bool_value(false));
    assert!(after.process.is_none());
}
#[test]
fn runtime_reset_preserves_complete_plant_and_restages_physical_feedback() {
    let service = RuntimeSessionService::default();
    let loaded = load(&service, EXAMPLE);
    let mut before = attach(&service, loaded.session_id);
    for _ in 0..3 {
        before = service.single_scan(target(before.session_id)).unwrap();
    }
    let reset = service.reset(target(before.session_id)).unwrap();
    assert_eq!(reset.process, before.process);
    assert_eq!(reset.completed_scans.get(), 0);
    assert_eq!(reset.simulation_time_micros.as_micros(), 0);
    assert_eq!(variable(&reset, "InletValve").value, bool_value(false));
    assert_eq!(variable(&reset, "LevelLow").value, bool_value(false));
    assert_eq!(
        variable(&reset, "LevelPercent").value,
        RuntimeValue::Real {
            value: PlcReal::new(30.0).unwrap()
        }
    );
}
#[test]
fn process_reset_preserves_plc_time_count_writable_memory_ton_and_user_inputs() {
    let source=EXAMPLE.replace("VAR_INPUT","VAR_INPUT User : BOOL;").replace("InletValve :=", "VAR Delay : TON; Count : INT; END_VAR\nDelay(IN := TRUE, PT := T#300ms); Count := Count + 1; InletValve :=");
    let service = RuntimeSessionService::default();
    let loaded = load(&service, &source);
    let attached = attach(&service, loaded.session_id);
    let mut before = input(&service, &attached, "User", bool_value(true));
    for _ in 0..4 {
        before = service.single_scan(target(before.session_id)).unwrap();
    }
    let reset = service.reset_process(target(before.session_id)).unwrap();
    assert_eq!(tank(&reset).level_permille, 0);
    assert!(tank(&reset).low_level);
    assert!(!tank(&reset).high_level);
    assert_eq!(
        tank(&reset).inlet_valve_open,
        tank(&before).inlet_valve_open
    );
    assert_eq!(reset.completed_scans, before.completed_scans);
    assert_eq!(reset.simulation_time_micros, before.simulation_time_micros);
    assert_eq!(writable(&reset), writable(&before));
    assert_eq!(reset.function_blocks, before.function_blocks);
    assert_eq!(variable(&reset, "User").value, bool_value(true));
    assert_eq!(variable(&reset, "LevelLow").value, bool_value(true));
    assert_eq!(
        variable(&reset, "LevelPercent").value,
        RuntimeValue::Real {
            value: PlcReal::new(0.0).unwrap()
        }
    );
}
#[test]
fn real_plc_fault_preserves_complete_plant_and_rolls_back_plc_committed_state_time_count() {
    let source=EXAMPLE.replace("VAR_INPUT","VAR_INPUT Fail : BOOL; Divisor : INT;").replace("InletValve :=", "VAR Result : INT; END_VAR\nIF Fail THEN InletValve := FALSE; DrainPump := TRUE; Result := 100 / Divisor; END_IF; InletValve :=");
    let service = RuntimeSessionService::default();
    let loaded = load(&service, &source);
    let attached = attach(&service, loaded.session_id);
    let s = service.single_scan(target(attached.session_id)).unwrap();
    let before = input(&service, &s, "Fail", bool_value(true));
    let fault = service.single_scan(target(before.session_id)).unwrap();
    assert_eq!(fault.state, SessionRuntimeState::Faulted);
    assert_eq!(fault.fault.as_ref().unwrap().code, "PLC-RUNTIME-003");
    assert_eq!(fault.process, before.process);
    assert_eq!(fault.completed_scans, before.completed_scans);
    assert_eq!(fault.simulation_time_micros, before.simulation_time_micros);
    assert_eq!(fault.variables, before.variables);
    assert_eq!(
        service
            .attach_process(AttachProcessSceneRequest {
                contract_version: CONTRACT_VERSION,
                session_id: fault.session_id,
                scene_kind: ProcessSceneKind::TankFilling
            })
            .unwrap_err()
            .code,
        "PLC-PROCESS-006"
    );
    assert_eq!(service.snapshot(target(fault.session_id)).unwrap(), fault);
    assert_eq!(
        service.single_scan(target(fault.session_id)).unwrap(),
        fault
    );
    let reset = service.reset(target(fault.session_id)).unwrap();
    assert_eq!(reset.process, fault.process);
    assert!(reset.fault.is_none());
}
#[test]
fn actual_source_controls_drain_and_clamps_without_a_level_setting_backdoor() {
    let source = EXAMPLE
        .replace("VAR_INPUT", "VAR_INPUT Drain : BOOL;")
        .replace(
            "InletValve := NOT LevelHigh;\nDrainPump := FALSE;",
            "InletValve := NOT Drain;\nDrainPump := Drain;",
        );
    let service = RuntimeSessionService::default();
    let loaded = load(&service, &source);
    let mut s = attach(&service, loaded.session_id);
    for _ in 0..10 {
        s = service.single_scan(target(s.session_id)).unwrap();
    }
    assert_eq!(tank(&s).level_permille, 1000);
    s = input(&service, &s, "Drain", bool_value(true));
    s = service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(tank(&s).level_permille, 920);
    assert!(tank(&s).drain_pump_running);
    assert!(!tank(&s).inlet_valve_open);
    for _ in 0..20 {
        s = service.single_scan(target(s.session_id)).unwrap();
    }
    assert_eq!(tank(&s).level_permille, 0);
    assert!(tank(&s).low_level);
}
#[test]
fn failed_load_preserves_attached_scene_and_successful_load_removes_it() {
    let service = RuntimeSessionService::default();
    let loaded = load(&service, EXAMPLE);
    let attached = attach(&service, loaded.session_id);
    let before = service.single_scan(target(attached.session_id)).unwrap();
    assert!(matches!(
        service
            .load(LoadStProgramRequest {
                contract_version: CONTRACT_VERSION,
                source: "bad".into()
            })
            .unwrap(),
        LoadProgramResponse::CompileFailed { .. }
    ));
    assert_eq!(service.snapshot(target(before.session_id)).unwrap(), before);
    let new = load(&service, EXAMPLE);
    assert!(new.process.is_none());
    assert_eq!(new.session_id.get(), 2);
    assert!(
        new.variables
            .iter()
            .filter(|v| v.role == RuntimeRole::Input)
            .all(|v| v.input_authority == Some(InputAuthority::User))
    );
}
#[test]
fn unload_drops_scene_and_replacement_never_reuses_session_identity() {
    let service = RuntimeSessionService::default();
    let loaded = load(&service, EXAMPLE);
    let s = attach(&service, loaded.session_id);
    service.unload(target(s.session_id)).unwrap();
    assert_eq!(
        service.snapshot(target(s.session_id)).unwrap_err().code,
        "APP-RUNTIME-STALE-SESSION"
    );
    let next = load(&service, EXAMPLE);
    assert_eq!(next.session_id.get(), 2);
    assert!(next.process.is_none());
}
#[test]
fn all_stale_scene_commands_reject_and_preserve_current_attached_session() {
    let service = RuntimeSessionService::default();
    let old = load(&service, EXAMPLE);
    let new = load(&service, EXAMPLE);
    let before = attach(&service, new.session_id);
    assert_eq!(
        service
            .attach_process(AttachProcessSceneRequest {
                contract_version: CONTRACT_VERSION,
                session_id: old.session_id,
                scene_kind: ProcessSceneKind::TankFilling
            })
            .unwrap_err()
            .code,
        "APP-RUNTIME-STALE-SESSION"
    );
    assert_eq!(
        service
            .reset_process(target(old.session_id))
            .unwrap_err()
            .code,
        "APP-RUNTIME-STALE-SESSION"
    );
    assert_eq!(
        service
            .detach_process(target(old.session_id))
            .unwrap_err()
            .code,
        "APP-RUNTIME-STALE-SESSION"
    );
    assert_eq!(service.snapshot(target(before.session_id)).unwrap(), before);
}
#[test]
fn failed_attachment_version_preserves_previous_valid_scene_and_runtime() {
    let service = RuntimeSessionService::default();
    let loaded = load(&service, EXAMPLE);
    let attached = attach(&service, loaded.session_id);
    let before = service.single_scan(target(attached.session_id)).unwrap();
    let invalid = RuntimeSessionRequest {
        contract_version: CONTRACT_VERSION - 1,
        session_id: before.session_id,
    };
    assert_eq!(
        service
            .attach_process(AttachProcessSceneRequest {
                contract_version: invalid.contract_version,
                session_id: before.session_id,
                scene_kind: ProcessSceneKind::TankFilling
            })
            .unwrap_err()
            .code,
        "APP-IPC-001"
    );
    assert_eq!(
        service.reset_process(invalid.clone()).unwrap_err().code,
        "APP-IPC-001"
    );
    assert_eq!(
        service.detach_process(invalid).unwrap_err().code,
        "APP-IPC-001"
    );
    assert_eq!(service.snapshot(target(before.session_id)).unwrap(), before);
}
#[test]
fn attachment_records_existing_committed_actuators_without_advancing_process_or_plc() {
    let service = RuntimeSessionService::default();
    let s = load(&service, EXAMPLE);
    let before = service.single_scan(target(s.session_id)).unwrap();
    let attached = attach(&service, before.session_id);
    assert_eq!(tank(&attached).level_permille, 0);
    assert!(tank(&attached).inlet_valve_open);
    assert_eq!(attached.completed_scans, before.completed_scans);
    assert_eq!(
        attached.simulation_time_micros,
        before.simulation_time_micros
    );
}
#[test]
fn attachment_to_faulted_runtime_is_rejected_and_reset_process_does_not_clear_fault() {
    // Use actual valid ST with a selected arithmetic fault, not malformed source.
    let fault_source = EXAMPLE.replace(
        "InletValve := NOT LevelHigh;",
        "VAR X : INT; END_VAR X := 1 / 0; InletValve := NOT LevelHigh;",
    );
    let service = RuntimeSessionService::default();
    let s = load(&service, &fault_source);
    let before = service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(
        service
            .attach_process(AttachProcessSceneRequest {
                contract_version: CONTRACT_VERSION,
                session_id: s.session_id,
                scene_kind: ProcessSceneKind::TankFilling
            })
            .unwrap_err()
            .code,
        "PLC-PROCESS-006"
    );
    assert_eq!(service.snapshot(target(s.session_id)).unwrap(), before);
    let s = load(&service, &fault_source);
    let attached = attach(&service, s.session_id);
    let fault = service.single_scan(target(attached.session_id)).unwrap();
    let reset = service.reset_process(target(fault.session_id)).unwrap();
    assert_eq!(reset.state, SessionRuntimeState::Faulted);
    assert_eq!(reset.fault, fault.fault);
    assert_eq!(reset.completed_scans, fault.completed_scans);
    assert_eq!(reset.simulation_time_micros, fault.simulation_time_micros);
}
#[test]
fn missing_scene_operations_are_clear_errors_without_mutating_loaded_runtime() {
    let service = RuntimeSessionService::default();
    let before = load(&service, EXAMPLE);
    assert_eq!(
        service
            .reset_process(target(before.session_id))
            .unwrap_err()
            .code,
        "PLC-PROCESS-005"
    );
    assert_eq!(
        service
            .detach_process(target(before.session_id))
            .unwrap_err()
            .code,
        "PLC-PROCESS-005"
    );
    assert_eq!(service.snapshot(target(before.session_id)).unwrap(), before);
}
#[test]
fn repeated_full_process_runs_produce_identical_combined_observations() {
    let run = || {
        let service = RuntimeSessionService::default();
        let s = load(&service, EXAMPLE);
        let a = attach(&service, s.session_id);
        let mut observations = vec![a];
        for _ in 0..10 {
            observations.push(service.single_scan(target(s.session_id)).unwrap());
        }
        observations
    };
    assert_eq!(run(), run());
}
