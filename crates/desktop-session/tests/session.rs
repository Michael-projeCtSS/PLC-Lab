use app_contract::*;
use desktop_session::RuntimeSessionService;
use plc_types::{PlcReal, PlcTime};
const SIMPLE: &str = "PROGRAM Main VAR_INPUT Start : BOOL; END_VAR VAR_OUTPUT Motor : BOOL; END_VAR VAR Internal : BOOL; END_VAR Internal := Start; Motor := Internal; END_PROGRAM";
const TON: &str = "PROGRAM Main VAR_INPUT Start : BOOL; END_VAR VAR_OUTPUT Motor : BOOL; END_VAR VAR Delay : TON; END_VAR Delay(IN := Start, PT := T#300ms, Q => Motor); END_PROGRAM";
const FAULT: &str = "PROGRAM Main\nVAR_INPUT Divisor : INT; END_VAR\nVAR_OUTPUT Result : INT; END_VAR\nResult := 100 / Divisor;\nEND_PROGRAM";
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
        panic!("Source must load")
    };
    *snapshot
}
fn input(
    service: &RuntimeSessionService,
    s: &RuntimeSnapshotDto,
    name: &str,
    value: RuntimeValue,
) -> RuntimeSnapshotDto {
    let variable_id = s
        .variables
        .iter()
        .find(|v| v.name == name)
        .unwrap()
        .variable_id;
    service
        .set_input(SetRuntimeInputRequest {
            contract_version: CONTRACT_VERSION,
            session_id: s.session_id,
            variable_id,
            value,
        })
        .unwrap()
}
fn value(s: &RuntimeSnapshotDto, name: &str) -> RuntimeValue {
    s.variables.iter().find(|v| v.name == name).unwrap().value
}
fn bool_value(value: bool) -> RuntimeValue {
    RuntimeValue::Bool { value }
}
#[test]
fn successful_load_creates_session_one() {
    let s = load(&RuntimeSessionService::default(), SIMPLE);
    assert_eq!(s.session_id.get(), 1);
    assert_eq!(s.program_name, "Main");
}
#[test]
fn replacement_and_unload_never_reuse_generation() {
    let service = RuntimeSessionService::default();
    let a = load(&service, SIMPLE);
    let b = load(&service, TON);
    assert_eq!(b.session_id.get(), 2);
    service.unload(target(b.session_id)).unwrap();
    let c = load(&service, SIMPLE);
    assert_eq!(c.session_id.get(), 3);
    assert_ne!(a.session_id, c.session_id);
}
#[test]
fn failed_replacement_preserves_complete_prior_session() {
    let service = RuntimeSessionService::default();
    let a = load(&service, SIMPLE);
    input(&service, &a, "Start", bool_value(true));
    let before = service.single_scan(target(a.session_id)).unwrap();
    let result = service
        .load(LoadStProgramRequest {
            contract_version: CONTRACT_VERSION,
            source: "PROGRAM Bad Missing := TRUE; END_PROGRAM".into(),
        })
        .unwrap();
    assert!(matches!(result, LoadProgramResponse::CompileFailed { .. }));
    assert_eq!(service.snapshot(target(a.session_id)).unwrap(), before);
    assert_eq!(load(&service, SIMPLE).session_id.get(), 2);
}
fn stale() -> (
    RuntimeSessionService,
    RuntimeSnapshotDto,
    RuntimeSnapshotDto,
) {
    let service = RuntimeSessionService::default();
    let a = load(&service, SIMPLE);
    let b = load(&service, SIMPLE);
    (service, a, b)
}
#[test]
fn stale_input_cannot_mutate_replacement() {
    let (service, a, b) = stale();
    let error = service
        .set_input(SetRuntimeInputRequest {
            contract_version: CONTRACT_VERSION,
            session_id: a.session_id,
            variable_id: a.variables[0].variable_id,
            value: bool_value(true),
        })
        .unwrap_err();
    assert_eq!(error.code, "APP-RUNTIME-STALE-SESSION");
    assert_eq!(service.snapshot(target(b.session_id)).unwrap(), b);
}
#[test]
fn stale_scan_cannot_mutate_replacement() {
    let (service, a, b) = stale();
    assert_eq!(
        service.single_scan(target(a.session_id)).unwrap_err().code,
        "APP-RUNTIME-STALE-SESSION"
    );
    assert_eq!(service.snapshot(target(b.session_id)).unwrap(), b);
}
#[test]
fn stale_reset_cannot_mutate_replacement() {
    let (service, a, b) = stale();
    assert_eq!(
        service.reset(target(a.session_id)).unwrap_err().code,
        "APP-RUNTIME-STALE-SESSION"
    );
    assert_eq!(service.snapshot(target(b.session_id)).unwrap(), b);
}
#[test]
fn stale_unload_and_observation_rejected() {
    let (service, a, b) = stale();
    assert_eq!(
        service.unload(target(a.session_id)).unwrap_err().code,
        "APP-RUNTIME-STALE-SESSION"
    );
    assert_eq!(
        service.snapshot(target(a.session_id)).unwrap_err().code,
        "APP-RUNTIME-STALE-SESSION"
    );
    assert_eq!(service.snapshot(target(b.session_id)).unwrap(), b);
}
#[test]
fn scalar_descriptors_have_source_names_types_roles_and_separate_fb_ids() {
    let s = load(
        &RuntimeSessionService::default(),
        "PROGRAM MiXeD VAR_INPUT Input : BOOL; END_VAR VAR_OUTPUT Output : INT; END_VAR VAR Local : REAL; Duration : TIME; Delay : TON; END_VAR END_PROGRAM",
    );
    assert_eq!(s.program_name, "MiXeD");
    assert_eq!(
        s.variables
            .iter()
            .map(|v| (v.variable_id.get(), v.name.as_str(), v.role))
            .collect::<Vec<_>>(),
        vec![
            (1, "Input", RuntimeRole::Input),
            (2, "Output", RuntimeRole::Output),
            (3, "Local", RuntimeRole::Local),
            (4, "Duration", RuntimeRole::Local)
        ]
    );
    assert_eq!(s.function_blocks.len(), 1);
    assert!(!s.variables.iter().any(|v| v.name == "Delay"));
}
#[test]
fn load_defaults_and_configuration_are_authoritative() {
    let s = load(
        &RuntimeSessionService::default(),
        "PROGRAM Main VAR_INPUT B:BOOL; I:INT; R:REAL; T:TIME; END_VAR VAR D:TON; END_VAR END_PROGRAM",
    );
    assert_eq!(s.state, SessionRuntimeState::Ready);
    assert_eq!(s.completed_scans.get(), 0);
    assert_eq!(s.simulation_time_micros, PlcTime::ZERO);
    assert_eq!(
        s.cycle_period_micros.as_micros(),
        desktop_session::DEFAULT_CYCLE_MICROS
    );
    assert_eq!(
        s.operation_budget.get(),
        desktop_session::DEFAULT_OPERATION_BUDGET
    );
    assert_eq!(value(&s, "B"), bool_value(false));
    assert_eq!(value(&s, "I"), RuntimeValue::Int { value: 0 });
    assert_eq!(
        value(&s, "R"),
        RuntimeValue::Real {
            value: PlcReal::new(0.0).unwrap()
        }
    );
    assert_eq!(
        value(&s, "T"),
        RuntimeValue::Time {
            microseconds: PlcTime::ZERO
        }
    );
}
#[test]
fn input_is_staged_without_output_or_clock_prediction() {
    let service = RuntimeSessionService::default();
    let s = load(&service, SIMPLE);
    let staged = input(&service, &s, "Start", bool_value(true));
    assert_eq!(value(&staged, "Start"), bool_value(true));
    assert_eq!(value(&staged, "Motor"), bool_value(false));
    assert_eq!(staged.completed_scans.get(), 0);
    assert_eq!(staged.simulation_time_micros, PlcTime::ZERO);
}
#[test]
fn scan_commits_authoritative_values_and_same_scan_read_after_write() {
    let service = RuntimeSessionService::default();
    let s = load(&service, SIMPLE);
    input(&service, &s, "Start", bool_value(true));
    let after = service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(value(&after, "Internal"), bool_value(true));
    assert_eq!(value(&after, "Motor"), bool_value(true));
    assert_eq!(after.completed_scans.get(), 1);
    assert_eq!(after.simulation_time_micros.as_micros(), 100_000);
    assert_eq!(after, service.snapshot(target(s.session_id)).unwrap());
}
#[test]
fn division_fault_is_domain_data_with_actual_source_span() {
    let service = RuntimeSessionService::default();
    let s = load(&service, FAULT);
    let after = service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(after.state, SessionRuntimeState::Faulted);
    let d = after.fault.as_ref().unwrap();
    assert_eq!(d.code, "PLC-RUNTIME-003");
    assert_eq!(d.category, "runtimeFault");
    let span = d.span.as_ref().unwrap();
    assert_eq!(span.start.line, 4);
    assert_eq!(span.start.column, 11);
    assert_eq!(span.end.column, 24);
    assert_eq!(d.related_variable.as_ref().unwrap().name, "Result");
}
#[test]
fn failed_scan_shows_rolled_back_committed_output_time_and_count() {
    let service = RuntimeSessionService::default();
    let s = load(&service, FAULT);
    input(&service, &s, "Divisor", RuntimeValue::Int { value: 2 });
    let before = service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(value(&before, "Result"), RuntimeValue::Int { value: 50 });
    input(&service, &s, "Divisor", RuntimeValue::Int { value: 0 });
    let fault = service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(value(&fault, "Result"), value(&before, "Result"));
    assert_eq!(fault.completed_scans, before.completed_scans);
    assert_eq!(fault.simulation_time_micros, before.simulation_time_micros);
    assert_eq!(service.single_scan(target(s.session_id)).unwrap(), fault);
}
#[test]
fn reset_reinitializes_real_runtime_inputs_memory_time_and_fault() {
    let service = RuntimeSessionService::default();
    let s = load(&service, FAULT);
    input(&service, &s, "Divisor", RuntimeValue::Int { value: 2 });
    service.single_scan(target(s.session_id)).unwrap();
    input(&service, &s, "Divisor", RuntimeValue::Int { value: 0 });
    service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(service.reset(target(s.session_id)).unwrap(), s);
}
#[test]
fn ton_four_scan_sequence_and_reset_use_existing_state() {
    let service = RuntimeSessionService::default();
    let s = load(&service, TON);
    input(&service, &s, "Start", bool_value(true));
    for (n, (q, et)) in [
        (false, 0),
        (false, 100_000),
        (false, 200_000),
        (true, 300_000),
    ]
    .into_iter()
    .enumerate()
    {
        let after = service.single_scan(target(s.session_id)).unwrap();
        assert_eq!(value(&after, "Motor"), bool_value(q));
        assert_eq!(after.completed_scans.get(), n as u64 + 1);
        assert_eq!(
            after.simulation_time_micros.as_micros(),
            (n as u64 + 1) * 100_000
        );
        let RuntimeFunctionBlock::Ton {
            q: actual,
            et_micros,
            ..
        } = after.function_blocks[0];
        assert_eq!(actual, q);
        assert_eq!(et_micros.as_micros(), et);
    }
    assert_eq!(service.reset(target(s.session_id)).unwrap(), s);
}
#[test]
fn ton_snapshot_hides_private_epoch_and_has_no_scalar_q_et_ids() {
    let s = load(&RuntimeSessionService::default(), TON);
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("active_since"));
    assert!(!json.contains("activeSince"));
    assert_eq!(s.variables.len(), 2);
    let block = serde_json::to_value(&s.function_blocks[0]).unwrap();
    assert_eq!(
        block
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["etMicros", "instanceId", "kind", "name", "q"]
    );
}
#[test]
fn unload_removes_runtime_and_old_generation_is_unusable() {
    let service = RuntimeSessionService::default();
    let s = load(&service, SIMPLE);
    let result = service.unload(target(s.session_id)).unwrap();
    assert_eq!(result.session_id, s.session_id);
    assert_eq!(
        service.snapshot(target(s.session_id)).unwrap_err().code,
        "APP-RUNTIME-STALE-SESSION"
    );
}
#[test]
fn unknown_wrong_role_and_wrong_type_inputs_leave_snapshot_unchanged() {
    let service = RuntimeSessionService::default();
    let s = load(&service, SIMPLE);
    for (variable_id, value) in [
        (RuntimeVariableId::new(999).unwrap(), bool_value(true)),
        (s.variables[1].variable_id, bool_value(true)),
        (s.variables[2].variable_id, bool_value(true)),
        (s.variables[0].variable_id, RuntimeValue::Int { value: 1 }),
    ] {
        assert_eq!(
            service
                .set_input(SetRuntimeInputRequest {
                    contract_version: CONTRACT_VERSION,
                    session_id: s.session_id,
                    variable_id,
                    value
                })
                .unwrap_err()
                .code,
            "APP-RUNTIME-INPUT"
        );
        assert_eq!(service.snapshot(target(s.session_id)).unwrap(), s);
    }
}
#[test]
fn maximum_time_and_finite_real_values_round_trip_through_real_session() {
    let service = RuntimeSessionService::default();
    let s = load(
        &service,
        "PROGRAM Main VAR_INPUT T:TIME; R:REAL; END_VAR VAR_OUTPUT OutTime:TIME; OutReal:REAL; END_VAR OutTime := T; OutReal := R; END_PROGRAM",
    );
    let t = RuntimeValue::Time {
        microseconds: PlcTime::from_micros(u64::MAX),
    };
    let r = RuntimeValue::Real {
        value: PlcReal::new(f32::from_bits(1)).unwrap(),
    };
    input(&service, &s, "T", t);
    input(&service, &s, "R", r);
    let result = service.single_scan(target(s.session_id)).unwrap();
    assert_eq!(value(&result, "OutTime"), t);
    assert_eq!(value(&result, "OutReal"), r);
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("18446744073709551615"));
    assert_eq!(
        serde_json::from_str::<RuntimeSnapshotDto>(&json).unwrap(),
        result
    );
}
#[test]
fn all_session_operations_reject_wrong_version_before_mutation() {
    let service = RuntimeSessionService::default();
    let s = load(&service, SIMPLE);
    let mut req = target(s.session_id);
    req.contract_version = CONTRACT_VERSION - 1;
    assert_eq!(
        service.single_scan(req.clone()).unwrap_err().code,
        "APP-IPC-001"
    );
    assert_eq!(service.reset(req.clone()).unwrap_err().code, "APP-IPC-001");
    assert_eq!(
        service.snapshot(req.clone()).unwrap_err().code,
        "APP-IPC-001"
    );
    assert_eq!(service.unload(req.clone()).unwrap_err().code, "APP-IPC-001");
    assert_eq!(
        service
            .set_input(SetRuntimeInputRequest {
                contract_version: req.contract_version,
                session_id: s.session_id,
                variable_id: s.variables[0].variable_id,
                value: bool_value(true)
            })
            .unwrap_err()
            .code,
        "APP-IPC-001"
    );
    assert_eq!(
        service
            .load(LoadStProgramRequest {
                contract_version: req.contract_version,
                source: SIMPLE.into()
            })
            .unwrap_err()
            .code,
        "APP-IPC-001"
    );
    assert_eq!(service.snapshot(target(s.session_id)).unwrap(), s);
}
#[test]
fn concurrent_scans_are_serialized_and_return_coherent_unique_observations() {
    use std::sync::{Arc, Barrier};
    let service = Arc::new(RuntimeSessionService::default());
    let s = load(&service, SIMPLE);
    let barrier = Arc::new(Barrier::new(8));
    let handles = (0..8)
        .map(|_| {
            let service = service.clone();
            let barrier = barrier.clone();
            let id = s.session_id;
            std::thread::spawn(move || {
                barrier.wait();
                service.single_scan(target(id)).unwrap()
            })
        })
        .collect::<Vec<_>>();
    let mut counts = handles
        .into_iter()
        .map(|h| {
            let s = h.join().unwrap();
            assert_eq!(
                s.simulation_time_micros.as_micros(),
                s.completed_scans.get() * 100_000
            );
            s.completed_scans.get()
        })
        .collect::<Vec<_>>();
    counts.sort();
    assert_eq!(counts, (1..=8).collect::<Vec<_>>());
}
#[test]
fn concurrent_replacement_cannot_apply_old_input_to_new_session() {
    use std::sync::{Arc, Barrier};
    for _ in 0..20 {
        let service = Arc::new(RuntimeSessionService::default());
        let old = load(&service, SIMPLE);
        let barrier = Arc::new(Barrier::new(2));
        let worker = {
            let service = service.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.set_input(SetRuntimeInputRequest {
                    contract_version: CONTRACT_VERSION,
                    session_id: old.session_id,
                    variable_id: old.variables[0].variable_id,
                    value: bool_value(true),
                })
            })
        };
        barrier.wait();
        let new = load(&service, SIMPLE);
        match worker.join().unwrap() {
            Ok(snapshot) => assert_eq!(snapshot.session_id.get(), 1),
            Err(error) => assert_eq!(error.code, "APP-RUNTIME-STALE-SESSION"),
        };
        assert_eq!(service.snapshot(target(new.session_id)).unwrap(), new);
    }
}
