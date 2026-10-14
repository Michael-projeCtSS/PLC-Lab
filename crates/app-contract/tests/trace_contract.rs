use app_contract::*;
#[test]
fn trace_request_ids_are_exact_and_unknown_fields_are_rejected() {
    let valid = format!(
        r#"{{"contractVersion":{},"sessionId":"18446744073709551615","afterSequence":"9007199254740993","limit":256}}"#,
        CONTRACT_VERSION
    );
    let r: TracePageRequest = serde_json::from_str(&valid).unwrap();
    assert_eq!(r.session_id.get(), u64::MAX);
    assert_eq!(r.after_sequence.unwrap().get(), 9007199254740993);
    assert_eq!(
        serde_json::to_value(r).unwrap()["sessionId"],
        "18446744073709551615"
    );
    for cursor in ["0", "01", "-1", "18446744073709551616"] {
        assert!(
            serde_json::from_str::<TracePageRequest>(&valid.replace("9007199254740993", cursor))
                .is_err()
        );
    }
    assert!(
        serde_json::from_str::<TracePageRequest>(&valid.replace("256}", "256,\"extra\":true}"))
            .is_err()
    );
    assert!(
        serde_json::from_str::<TracePageRequest>(
            &valid.replace("\"9007199254740993\"", "9007199254740993")
        )
        .is_err()
    );
}
#[test]
fn event_kinds_are_closed_and_contract_version_is_incremented() {
    for kind in [
        "SessionLoaded",
        "InputStaged",
        "ProcessAttached",
        "ScanCompleted",
        "ScanFaulted",
        "RuntimeReset",
        "ProcessReset",
        "ProcessDetached",
    ] {
        let e: TraceEventKind = serde_json::from_str(&format!("\"{kind}\"")).unwrap();
        assert_eq!(serde_json::to_string(&e).unwrap(), format!("\"{kind}\""));
    }
    assert!(serde_json::from_str::<TraceEventKind>("\"Replay\"").is_err());
    assert_eq!(CONTRACT_VERSION, 6);
    assert_eq!(TRACE_PAGE_COMMAND, "get_trace_page");
}
