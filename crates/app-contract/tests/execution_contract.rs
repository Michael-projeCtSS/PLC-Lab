use app_contract::*;
#[test]
fn execution_modes_are_closed_and_wake_ids_are_strict_read_only_decimal_transport() {
    for (mode, text) in [
        (ExecutionMode::Stopped, "STOPPED"),
        (ExecutionMode::Running, "RUNNING"),
        (ExecutionMode::Paused, "PAUSED"),
    ] {
        assert_eq!(serde_json::to_string(&mode).unwrap(), format!("\"{text}\""));
        assert_eq!(
            serde_json::from_str::<ExecutionMode>(&format!("\"{text}\"")).unwrap(),
            mode
        );
    }
    for value in ["\"running\"", "\"FAULTED\"", "null", "7"] {
        assert!(serde_json::from_str::<ExecutionMode>(value).is_err());
    }
    let wake = ExecutionWake {
        contract_version: CONTRACT_VERSION,
        session_id: RuntimeSessionId::new(u64::MAX).unwrap(),
        run_id: ExecutionRunId::new(u64::MAX).unwrap(),
    };
    let json = serde_json::to_value(&wake).unwrap();
    assert_eq!(json["runId"], u64::MAX.to_string());
    assert_eq!(
        serde_json::from_value::<ExecutionWake>(json.clone()).unwrap(),
        wake
    );
    for value in [
        serde_json::json!(0),
        serde_json::json!("0"),
        serde_json::json!("01"),
        serde_json::json!("18446744073709551616"),
    ] {
        let mut broken = json.clone();
        broken["runId"] = value;
        assert!(serde_json::from_value::<ExecutionWake>(broken).is_err());
    }
    let mut bad = json;
    bad["snapshot"] = serde_json::json!({});
    assert!(serde_json::from_value::<ExecutionWake>(bad).is_err());
}
#[test]
fn control_targets_never_accept_epoch_or_state() {
    let valid = serde_json::json!({"contractVersion":CONTRACT_VERSION,"sessionId":"1"});
    assert!(serde_json::from_value::<RuntimeSessionRequest>(valid.clone()).is_ok());
    for field in ["epoch", "runId", "executionMode", "tick", "snapshot"] {
        let mut value = valid.clone();
        value[field] = serde_json::json!("1");
        assert!(serde_json::from_value::<RuntimeSessionRequest>(value).is_err());
    }
    assert_eq!(START_EXECUTION_COMMAND, "start_execution");
    assert_eq!(PAUSE_EXECUTION_COMMAND, "pause_execution");
    assert_eq!(STOP_EXECUTION_COMMAND, "stop_execution");
    assert_eq!(CONTRACT_VERSION, 7);
}
