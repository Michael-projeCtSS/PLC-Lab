use app_contract::*;
fn tank(level: u16) -> ProcessSnapshotDto {
    ProcessSnapshotDto::TankFilling(TankProcessSnapshotDto {
        level_permille: level,
        low_level: false,
        high_level: true,
        inlet_valve_open: true,
        drain_pump_running: false,
        low_threshold_permille: 200,
        high_threshold_permille: 800,
        fill_rate_permille_per_second: 1000,
        drain_rate_permille_per_second: 800,
    })
}
#[test]
fn tank_snapshot_round_trips_exact_shape_and_bounds() {
    for level in [0, 33, 800, 1000] {
        let dto = tank(level);
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["kind"], "TANK_FILLING");
        assert_eq!(
            serde_json::from_value::<ProcessSnapshotDto>(value).unwrap(),
            dto
        );
    }
}
#[test]
fn tank_snapshot_rejects_invalid_levels_thresholds_and_scalar_types() {
    for field in [
        "levelPermille",
        "lowThresholdPermille",
        "highThresholdPermille",
    ] {
        for invalid in [
            serde_json::json!(-1),
            serde_json::json!(1001),
            serde_json::json!(1.5),
            serde_json::json!("800"),
            serde_json::json!(null),
        ] {
            let mut value = serde_json::to_value(tank(800)).unwrap();
            value[field] = invalid;
            assert!(
                serde_json::from_value::<ProcessSnapshotDto>(value).is_err(),
                "{field}"
            );
        }
    }
    for field in [
        "lowLevel",
        "highLevel",
        "inletValveOpen",
        "drainPumpRunning",
    ] {
        let mut value = serde_json::to_value(tank(800)).unwrap();
        value[field] = serde_json::json!(1);
        assert!(serde_json::from_value::<ProcessSnapshotDto>(value).is_err());
    }
}
#[test]
fn process_kind_missing_fields_and_private_implementation_data_are_rejected() {
    let mut unknown = serde_json::to_value(tank(800)).unwrap();
    unknown["kind"] = serde_json::json!("OTHER");
    assert!(serde_json::from_value::<ProcessSnapshotDto>(unknown).is_err());
    for field in [
        "levelPermille",
        "lowLevel",
        "inletValveOpen",
        "fillRatePermillePerSecond",
    ] {
        let mut value = serde_json::to_value(tank(800)).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ProcessSnapshotDto>(value).is_err());
    }
    let mut private = serde_json::to_value(tank(800)).unwrap();
    private["remainder"] = serde_json::json!(1);
    assert!(serde_json::from_value::<ProcessSnapshotDto>(private).is_err());
}
#[test]
fn attach_request_is_strict_and_targeted_without_runtime_objects() {
    let request = AttachProcessSceneRequest {
        contract_version: CONTRACT_VERSION,
        session_id: RuntimeSessionId::new(1).unwrap(),
        scene_kind: ProcessSceneKind::TankFilling,
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(
        serde_json::from_value::<AttachProcessSceneRequest>(value).unwrap(),
        request
    );
    for source in [
        r#"{}"#,
        r#"{"contractVersion":5,"sessionId":"1","sceneKind":"OTHER"}"#,
        r#"{"contractVersion":5,"sessionId":"1","sceneKind":"TANK_FILLING","levelPermille":500}"#,
        r#"{"contractVersion":5,"sessionId":1,"sceneKind":"TANK_FILLING"}"#,
    ] {
        assert!(serde_json::from_str::<AttachProcessSceneRequest>(source).is_err());
    }
}
#[test]
fn input_authority_is_explicit_and_rejects_unknown_kinds() {
    assert_eq!(serde_json::to_value(InputAuthority::User).unwrap(), "USER");
    assert_eq!(
        serde_json::to_value(InputAuthority::Process).unwrap(),
        "PROCESS"
    );
    assert!(serde_json::from_str::<InputAuthority>(r#""OUTPUT""#).is_err());
    assert!(serde_json::from_str::<InputAuthority>("true").is_err());
}
