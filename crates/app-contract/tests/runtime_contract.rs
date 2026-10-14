use app_contract::*;
use plc_types::{PlcReal, PlcTime};
#[test]
fn typed_values_round_trip_without_time_or_real_loss() {
    for value in [
        RuntimeValue::Bool { value: true },
        RuntimeValue::Int { value: i16::MIN },
        RuntimeValue::Real {
            value: PlcReal::new(f32::MAX).unwrap(),
        },
        RuntimeValue::Real {
            value: PlcReal::new(f32::from_bits(1)).unwrap(),
        },
        RuntimeValue::Time {
            microseconds: PlcTime::from_micros(u64::MAX),
        },
    ] {
        let text = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<RuntimeValue>(&text).unwrap(), value);
        assert!(!text.contains("null"));
    }
}
#[test]
fn typed_values_reject_invalid_discriminants_ranges_nonfinite_and_fields() {
    for text in [
        r#"{"type":"BOOL","value":1}"#,
        r#"{"type":"INT","value":32768}"#,
        r#"{"type":"REAL","value":1e100}"#,
        r#"{"type":"REAL","value":"NaN"}"#,
        r#"{"type":"TIME","microseconds":9007199254740993}"#,
        r#"{"type":"TIME","microseconds":"18446744073709551616"}"#,
        r#"{"type":"TIME","microseconds":"01"}"#,
        r#"{"type":"TIME","microseconds":"-1"}"#,
        r#"{"type":"DINT","value":1}"#,
        r#"{"type":"BOOL","value":true,"extra":1}"#,
    ] {
        assert!(
            serde_json::from_str::<RuntimeValue>(text).is_err(),
            "{text}"
        );
    }
}
#[test]
fn real_construction_cannot_generate_nonfinite_json() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(PlcReal::new(value).is_err());
    }
    let value = RuntimeValue::Real {
        value: PlcReal::new(-0.0).unwrap(),
    };
    assert_eq!(serde_json::to_value(value).unwrap()["value"], 0.0);
}
#[test]
fn canonical_opaque_ids_reject_malformed_and_numeric_transport() {
    for text in [
        "0",
        "01",
        "-1",
        "+1",
        " 1",
        "1 ",
        "1.0",
        "18446744073709551616",
        "",
    ] {
        assert!(
            serde_json::from_str::<RuntimeSessionId>(&serde_json::to_string(text).unwrap())
                .is_err(),
            "{text}"
        );
    }
    assert!(serde_json::from_str::<RuntimeSessionId>("1").is_err());
    assert!(RuntimeVariableId::try_from("4294967296".to_string()).is_err());
    assert_eq!(
        RuntimeSessionId::try_from(u64::MAX.to_string())
            .unwrap()
            .get(),
        u64::MAX
    );
}
#[test]
fn session_and_input_requests_reject_extra_missing_and_invalid_fields() {
    for text in [
        r#"{}"#,
        r#"{"contractVersion":4,"sessionId":"1","extra":true}"#,
        r#"{"contractVersion":4,"sessionId":1}"#,
        r#"{"contractVersion":4,"sessionId":"0"}"#,
    ] {
        assert!(serde_json::from_str::<RuntimeSessionRequest>(text).is_err());
    }
    assert!(serde_json::from_str::<SetRuntimeInputRequest>(r#"{"contractVersion":4,"sessionId":"1","variableId":"0","value":{"type":"BOOL","value":true}}"#).is_err());
    assert!(
        serde_json::from_str::<LoadStProgramRequest>(
            r#"{"contractVersion":4,"source":"","cyclePeriodMicros":"1"}"#
        )
        .is_err()
    );
}
#[test]
fn load_and_unload_contract_round_trip() {
    let req = LoadStProgramRequest {
        contract_version: CONTRACT_VERSION,
        source: "PROGRAM Main END_PROGRAM".into(),
    };
    assert_eq!(
        serde_json::from_str::<LoadStProgramRequest>(&serde_json::to_string(&req).unwrap())
            .unwrap(),
        req
    );
    let unload = UnloadProgramResponse {
        contract_version: CONTRACT_VERSION,
        session_id: RuntimeSessionId::new(1).unwrap(),
    };
    assert_eq!(
        serde_json::from_str::<UnloadProgramResponse>(&serde_json::to_string(&unload).unwrap())
            .unwrap(),
        unload
    );
    assert!(
        serde_json::from_str::<LoadProgramResponse>(r#"{"kind":"compiled","diagnostics":[]}"#)
            .is_err()
    );
}
#[test]
fn ton_dto_rejects_private_epoch_and_extra_fields() {
    assert!(serde_json::from_str::<RuntimeFunctionBlock>(r#"{"kind":"TON","instanceId":"1","name":"Delay","q":false,"etMicros":"0","activeSince":"0"}"#).is_err());
}
