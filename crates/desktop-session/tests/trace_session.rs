use app_contract::*;
use desktop_session::RuntimeSessionService;
const TANK: &str = include_str!("../../../scenes/tank-filling/example.st");
fn load(s: &RuntimeSessionService, source: &str) -> RuntimeSnapshotDto {
    let LoadProgramResponse::Loaded { snapshot } = s
        .load(LoadStProgramRequest {
            contract_version: CONTRACT_VERSION,
            source: source.into(),
        })
        .unwrap()
    else {
        panic!("Real test source must compile")
    };
    *snapshot
}
fn target(s: &RuntimeSnapshotDto) -> RuntimeSessionRequest {
    RuntimeSessionRequest {
        contract_version: CONTRACT_VERSION,
        session_id: s.session_id,
    }
}
fn page(
    s: &RuntimeSessionService,
    id: RuntimeSessionId,
    after: Option<u64>,
    limit: u16,
) -> TracePageDto {
    s.trace_page(TracePageRequest {
        contract_version: CONTRACT_VERSION,
        session_id: id,
        after_sequence: after.map(|n| TraceSequenceId::new(n).unwrap()),
        limit,
    })
    .unwrap()
}
fn equal_boundary(e: &TraceEntryDto, s: &RuntimeSnapshotDto) {
    assert_eq!(e.completed_scans, s.completed_scans);
    assert_eq!(e.simulation_time_micros, s.simulation_time_micros);
    assert_eq!(e.runtime_state, s.state);
    assert_eq!(e.variables, s.variables);
    assert_eq!(e.function_blocks, s.function_blocks);
    assert_eq!(e.process, s.process);
    assert_eq!(e.fault, s.fault);
}
fn attach(s: &RuntimeSessionService, id: RuntimeSessionId) -> RuntimeSnapshotDto {
    s.attach_process(AttachProcessSceneRequest {
        contract_version: CONTRACT_VERSION,
        session_id: id,
        scene_kind: ProcessSceneKind::TankFilling,
    })
    .unwrap()
}
#[test]
fn loaded_and_post_staged_input_observations_are_exact_and_reads_never_append() {
    let s = RuntimeSessionService::default();
    let loaded = load(
        &s,
        "PROGRAM Main VAR_INPUT Enable:BOOL; END_VAR VAR_OUTPUT Motor:BOOL; END_VAR Motor:=Enable; END_PROGRAM",
    );
    let first = page(&s, loaded.session_id, None, 256);
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.entries[0].kind, TraceEventKind::SessionLoaded);
    equal_boundary(&first.entries[0], &loaded);
    let v = &loaded.variables[0];
    let staged = s
        .set_input(SetRuntimeInputRequest {
            contract_version: CONTRACT_VERSION,
            session_id: loaded.session_id,
            variable_id: v.variable_id,
            value: RuntimeValue::Bool { value: true },
        })
        .unwrap();
    let next = page(&s, loaded.session_id, Some(1), 1);
    assert_eq!(next.entries[0].kind, TraceEventKind::InputStaged);
    assert_eq!(
        next.entries[0].staged_input,
        Some(RuntimeRelatedVariable {
            variable_id: v.variable_id,
            name: v.name.clone()
        })
    );
    equal_boundary(&next.entries[0], &staged);
    assert_eq!(staged.completed_scans.get(), 0);
    assert_eq!(staged.simulation_time_micros.as_micros(), 0);
    s.snapshot(target(&loaded)).unwrap();
    assert_eq!(page(&s, loaded.session_id, Some(2), 256).entries.len(), 0);
    assert_eq!(page(&s, loaded.session_id, None, 256).entries.len(), 2);
}
#[test]
fn actual_tank_trace_has_exact_full_cycle_states_including_scan_eight_and_nine() {
    let s = RuntimeSessionService::default();
    let loaded = load(&s, TANK);
    let attached = attach(&s, loaded.session_id);
    let mut live = vec![loaded.clone(), attached];
    for _ in 0..9 {
        live.push(s.single_scan(target(&loaded)).unwrap());
    }
    let p = page(&s, loaded.session_id, None, 256);
    assert_eq!(p.entries.len(), 11);
    assert_eq!(p.entries[1].kind, TraceEventKind::ProcessAttached);
    for (i, (e, s)) in p.entries.iter().zip(&live).enumerate() {
        assert_eq!(e.sequence.get(), i as u64 + 1);
        equal_boundary(e, s);
        if i >= 2 {
            assert_eq!(e.kind, TraceEventKind::ScanCompleted);
        }
    }
    for (index, inlet) in [(9, true), (10, false)] {
        let e = &p.entries[index];
        let Some(ProcessSnapshotDto::TankFilling(t)) = &e.process else {
            panic!("Tank observation")
        };
        assert_eq!(t.level_permille, 800);
        assert!(t.high_level);
        assert_eq!(t.inlet_valve_open, inlet);
        assert_eq!(e.completed_scans.get(), index as u64 - 1);
        assert_eq!(
            e.simulation_time_micros.as_micros(),
            (index as u64 - 1) * 100000
        );
    }
    assert_eq!(
        page(&s, loaded.session_id, None, 3)
            .entries
            .iter()
            .map(|e| e.sequence.get())
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        page(&s, loaded.session_id, Some(3), 3)
            .entries
            .iter()
            .map(|e| e.sequence.get())
            .collect::<Vec<_>>(),
        vec![4, 5, 6]
    );
}
#[test]
fn genuine_fault_has_rollback_source_provenance_and_only_one_attempt_event() {
    let s = RuntimeSessionService::default();
    let loaded = load(
        &s,
        "PROGRAM Main VAR_INPUT Divisor:INT; END_VAR VAR_OUTPUT Result:INT; END_VAR Result:=100 / Divisor; END_PROGRAM",
    );
    let fault = s.single_scan(target(&loaded)).unwrap();
    let p = page(&s, loaded.session_id, None, 256);
    assert_eq!(p.entries.len(), 2);
    let e = &p.entries[1];
    assert_eq!(e.kind, TraceEventKind::ScanFaulted);
    equal_boundary(e, &fault);
    let d = e.fault.as_ref().unwrap();
    assert_eq!(d.code, "PLC-RUNTIME-003");
    assert_eq!(d.category, "runtimeFault");
    assert!(d.span.is_some());
    let span = d.span.as_ref().unwrap();
    assert_eq!(span.start.line, 1);
    assert_eq!(span.end.line, 1);
    assert_eq!(
        span.end.column - span.start.column,
        "100 / Divisor".len() as u32
    );
    assert_eq!(e.completed_scans.get(), 0);
    assert_eq!(e.simulation_time_micros.as_micros(), 0);
    assert_eq!(
        e.variables
            .iter()
            .find(|v| v.name == "Result")
            .unwrap()
            .value,
        RuntimeValue::Int { value: 0 }
    );
    s.single_scan(target(&loaded)).unwrap();
    assert_eq!(page(&s, loaded.session_id, None, 256), p);
    let reset = s.reset(target(&loaded)).unwrap();
    let after = page(&s, loaded.session_id, Some(2), 256);
    assert_eq!(after.entries.len(), 1);
    assert_eq!(after.entries[0].kind, TraceEventKind::RuntimeReset);
    equal_boundary(&after.entries[0], &reset);
    assert_eq!(reset.state, SessionRuntimeState::Ready);
    assert!(reset.fault.is_none());
}
#[test]
fn process_lifecycle_and_both_resets_have_distinct_post_action_observations() {
    let s = RuntimeSessionService::default();
    let loaded = load(&s, TANK);
    attach(&s, loaded.session_id);
    let cycle = s.single_scan(target(&loaded)).unwrap();
    let runtime = s.reset(target(&loaded)).unwrap();
    assert_eq!(runtime.process, cycle.process);
    assert_eq!(runtime.completed_scans.get(), 0);
    let process = s.reset_process(target(&loaded)).unwrap();
    let detached = s.detach_process(target(&loaded)).unwrap();
    let p = page(&s, loaded.session_id, None, 256);
    assert_eq!(
        p.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![
            TraceEventKind::SessionLoaded,
            TraceEventKind::ProcessAttached,
            TraceEventKind::ScanCompleted,
            TraceEventKind::RuntimeReset,
            TraceEventKind::ProcessReset,
            TraceEventKind::ProcessDetached
        ]
    );
    equal_boundary(&p.entries[3], &runtime);
    equal_boundary(&p.entries[4], &process);
    equal_boundary(&p.entries[5], &detached);
    assert!(detached.process.is_none());
    assert!(
        detached
            .variables
            .iter()
            .filter(|v| v.role == RuntimeRole::Input)
            .all(|v| v.input_authority == Some(InputAuthority::User))
    );
}
#[test]
fn failed_operations_preserve_history_and_replacement_unload_never_transfer_it() {
    let s = RuntimeSessionService::default();
    let loaded = load(&s, TANK);
    let before = page(&s, loaded.session_id, None, 256);
    assert!(s.reset_process(target(&loaded)).is_err());
    assert!(s.detach_process(target(&loaded)).is_err());
    assert!(matches!(
        s.load(LoadStProgramRequest {
            contract_version: CONTRACT_VERSION,
            source: "invalid".into()
        })
        .unwrap(),
        LoadProgramResponse::CompileFailed { .. }
    ));
    assert!(
        s.set_input(SetRuntimeInputRequest {
            contract_version: CONTRACT_VERSION,
            session_id: loaded.session_id,
            variable_id: loaded.variables[0].variable_id,
            value: RuntimeValue::Int { value: 5 }
        })
        .is_err()
    );
    assert!(
        s.single_scan(RuntimeSessionRequest {
            contract_version: CONTRACT_VERSION,
            session_id: RuntimeSessionId::new(999).unwrap()
        })
        .is_err()
    );
    assert_eq!(page(&s, loaded.session_id, None, 256), before);
    attach(&s, loaded.session_id);
    let attached = page(&s, loaded.session_id, None, 256);
    assert!(
        s.set_input(SetRuntimeInputRequest {
            contract_version: CONTRACT_VERSION,
            session_id: loaded.session_id,
            variable_id: loaded.variables[0].variable_id,
            value: RuntimeValue::Bool { value: true }
        })
        .is_err()
    );
    assert_eq!(page(&s, loaded.session_id, None, 256), attached);
    let next = load(&s, "PROGRAM Other END_PROGRAM");
    assert_ne!(next.session_id, loaded.session_id);
    assert_eq!(page(&s, next.session_id, None, 256).entries.len(), 1);
    assert_eq!(
        page(&s, next.session_id, None, 256).entries[0]
            .sequence
            .get(),
        1
    );
    assert!(
        s.trace_page(TracePageRequest {
            contract_version: CONTRACT_VERSION,
            session_id: loaded.session_id,
            after_sequence: None,
            limit: 256
        })
        .is_err()
    );
    let new_before = page(&s, next.session_id, None, 256);
    assert!(
        s.attach_process(AttachProcessSceneRequest {
            contract_version: CONTRACT_VERSION,
            session_id: next.session_id,
            scene_kind: ProcessSceneKind::TankFilling
        })
        .is_err()
    );
    assert_eq!(page(&s, next.session_id, None, 256), new_before);
    s.unload(target(&next)).unwrap();
    assert!(
        s.trace_page(TracePageRequest {
            contract_version: CONTRACT_VERSION,
            session_id: next.session_id,
            after_sequence: None,
            limit: 256
        })
        .is_err()
    );
}
#[test]
fn invalid_version_limits_and_future_cursors_fail_without_recording() {
    let s = RuntimeSessionService::default();
    let loaded = load(&s, TANK);
    let before = page(&s, loaded.session_id, None, 256);
    for limit in [0, 257, u16::MAX] {
        assert_eq!(
            s.trace_page(TracePageRequest {
                contract_version: CONTRACT_VERSION,
                session_id: loaded.session_id,
                after_sequence: None,
                limit
            })
            .unwrap_err()
            .code,
            "APP-TRACE-LIMIT"
        );
    }
    assert_eq!(
        s.trace_page(TracePageRequest {
            contract_version: CONTRACT_VERSION,
            session_id: loaded.session_id,
            after_sequence: Some(TraceSequenceId::new(2).unwrap()),
            limit: 1
        })
        .unwrap_err()
        .code,
        "APP-TRACE-CURSOR"
    );
    assert_eq!(
        s.trace_page(TracePageRequest {
            contract_version: 0,
            session_id: loaded.session_id,
            after_sequence: None,
            limit: 1
        })
        .unwrap_err()
        .code,
        "APP-IPC-001"
    );
    assert_eq!(page(&s, loaded.session_id, None, 256), before);
}
#[test]
fn full_trace_determinism_includes_ton_fault_process_and_exact_reset_history() {
    const SOURCE: &str = "PROGRAM Main VAR_INPUT LevelLow:BOOL; LevelHigh:BOOL; LevelPercent:REAL; Start:BOOL; Divisor:INT; END_VAR VAR_OUTPUT InletValve:BOOL; DrainPump:BOOL; END_VAR VAR Timer:TON; Result:INT; END_VAR Timer(IN:=Start, PT:=T#300ms); Result:=100 / Divisor; InletValve:=NOT LevelHigh; DrainPump:=FALSE; END_PROGRAM";
    let run = || {
        let s = RuntimeSessionService::default();
        let loaded = load(&s, SOURCE);
        let input = |name: &str, value| {
            s.set_input(SetRuntimeInputRequest {
                contract_version: CONTRACT_VERSION,
                session_id: loaded.session_id,
                variable_id: loaded
                    .variables
                    .iter()
                    .find(|v| v.name == name)
                    .unwrap()
                    .variable_id,
                value,
            })
            .unwrap()
        };
        input("Divisor", RuntimeValue::Int { value: 1 });
        input("Start", RuntimeValue::Bool { value: true });
        attach(&s, loaded.session_id);
        for _ in 0..9 {
            s.single_scan(target(&loaded)).unwrap();
        }
        input("Divisor", RuntimeValue::Int { value: 0 });
        s.single_scan(target(&loaded)).unwrap();
        s.reset(target(&loaded)).unwrap();
        s.reset_process(target(&loaded)).unwrap();
        s.detach_process(target(&loaded)).unwrap();
        page(&s, loaded.session_id, None, 256)
    };
    let a = run();
    let b = run();
    assert_eq!(a, b);
    assert!(a.entries.iter().any(|e| e.fault.is_some()));
    let fault = a
        .entries
        .iter()
        .find(|e| e.kind == TraceEventKind::ScanFaulted)
        .unwrap();
    let committed = a
        .entries
        .iter()
        .rev()
        .find(|e| e.kind == TraceEventKind::ScanCompleted)
        .unwrap();
    assert_eq!(fault.process, committed.process);
    assert_eq!(fault.completed_scans.get(), 9);
    assert_eq!(fault.simulation_time_micros.as_micros(), 900000);
    assert!(a.entries.iter().any(|e|e.function_blocks.iter().any(|b|matches!(b,RuntimeFunctionBlock::Ton{q:true,et_micros,..} if et_micros.as_micros()==300000))));
}
