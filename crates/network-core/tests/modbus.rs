use lab_model::{
    Identifier,
    network::{InterfaceConfig, Ipv4, ModbusTcpScan, NetworkInstallation},
};
use network_core::{
    NetworkEventKind, NetworkRuntime, PingResult, QuantitySource, ScanStatus, ScanValue, exception,
};
use simulation_core::WorldTime;

struct Level(i64);
impl QuantitySource for Level {
    fn quantity(&self, component: &str, name: &str) -> Option<i64> {
        (component == "tank" && name == "levelPermille").then_some(self.0)
    }
}

fn installation() -> NetworkInstallation {
    serde_json::from_str(
        r#"{"latencyUs": 500, "devices": [
          {"id": "lt-gw", "label": "Level transmitter", "interface": {"ip": "192.168.0.20", "mask": "255.255.255.0"},
           "role": {"type": "modbusTcpServer", "unitIds": {"type": "ignore"}, "processingUs": 2000,
             "holdingRegisters": [
               {"address": 0, "value": {"type": "scaledInt16", "component": "tank", "quantity": "levelPermille", "scale": 1, "divisor": 1}},
               {"address": 1, "value": {"type": "float32", "component": "tank", "quantity": "levelPermille", "divisor": 10, "wordOrder": "highWordFirst"}}
             ]}},
          {"id": "hmi", "label": "HMI", "interface": {"ip": "192.168.0.30", "mask": "255.255.255.0"}, "role": {"type": "host"}}
        ]}"#,
    )
    .unwrap()
}
fn scan(json: &str) -> ModbusTcpScan {
    serde_json::from_str(json).unwrap()
}
fn ip(s: &str) -> Ipv4 {
    Ipv4::parse(s).unwrap()
}
fn plc(addr: &str) -> Option<InterfaceConfig> {
    Some(InterfaceConfig {
        ip: ip(addr),
        mask: ip("255.255.255.0"),
        gateway: None,
    })
}
const GOOD: &str = r#"{"id": "level", "server": "192.168.0.20", "unitId": 1, "function": "readHoldingRegisters",
  "startAddress": 0, "quantity": 3, "periodMs": 100, "timeoutMs": 300,
  "values": [{"offset": 0, "dataType": {"type": "int16"}, "tag": "LevelInt"},
             {"offset": 1, "dataType": {"type": "float32", "wordOrder": "highWordFirst"}, "tag": "LevelReal"}]}"#;

fn run(net: &mut NetworkRuntime, until_ms: u64, level: i64) {
    let source = Level(level);
    while net.process_next(WorldTime::from_micros(until_ms * 1000), &source) {}
}

#[test]
fn healthy_scan_reads_int_and_float_registers_with_round_trip() {
    let mut net = NetworkRuntime::new(Some(&installation()));
    net.configure(plc("192.168.0.10"), &[scan(GOOD)]);
    run(&mut net, 50, 625);
    assert_eq!(net.scan_status(0), Some(ScanStatus::Ok));
    assert_eq!(net.value(0, 0), Some(ScanValue::Int(625)));
    assert_eq!(net.value(0, 1), Some(ScanValue::Real(62.5)));
    let events = net.drain_events();
    let rtt = events.iter().find_map(|e| match e.kind {
        NetworkEventKind::ResponseReceived { round_trip_us } => Some(round_trip_us),
        _ => None,
    });
    // Connection setup (2 x latency) + request (latency) + processing + response (latency).
    assert_eq!(rtt, Some(1000 + 500 + 2000 + 500));
    assert_eq!(
        net.ping(ip("192.168.0.20")),
        PingResult::Reply {
            round_trip_us: 1000
        }
    );
}

#[test]
fn wrong_subnet_has_no_route_and_unknown_host_times_out() {
    let mut net = NetworkRuntime::new(Some(&installation()));
    net.configure(plc("192.168.1.10"), &[scan(GOOD)]);
    run(&mut net, 1000, 500);
    assert_eq!(net.scan_status(0), Some(ScanStatus::NoRoute));
    assert_eq!(net.ping(ip("192.168.0.20")), PingResult::NoRoute);

    let mut net = NetworkRuntime::new(Some(&installation()));
    net.configure(
        plc("192.168.0.10"),
        &[scan(&GOOD.replace("192.168.0.20", "192.168.0.21"))],
    );
    run(&mut net, 1000, 500);
    assert_eq!(net.scan_status(0), Some(ScanStatus::Timeout));
    assert!(
        net.drain_events()
            .iter()
            .any(|e| matches!(e.kind, NetworkEventKind::ArpUnresolved { .. }))
    );
}

#[test]
fn host_without_server_refuses_and_offset_errors_return_exceptions() {
    let mut net = NetworkRuntime::new(Some(&installation()));
    net.configure(
        plc("192.168.0.10"),
        &[scan(&GOOD.replace("192.168.0.20", "192.168.0.30"))],
    );
    run(&mut net, 50, 500);
    assert_eq!(net.scan_status(0), Some(ScanStatus::ConnectionRefused));

    let mut net = NetworkRuntime::new(Some(&installation()));
    net.configure(
        plc("192.168.0.10"),
        &[scan(
            &GOOD.replace("\"startAddress\": 0", "\"startAddress\": 1"),
        )],
    );
    run(&mut net, 50, 500);
    assert_eq!(
        net.scan_status(0),
        Some(ScanStatus::Exception(exception::ILLEGAL_DATA_ADDRESS))
    );
}

#[test]
fn unit_identifier_is_checked_only_by_devices_that_route_by_unit() {
    // A device that ignores the unit identifier answers any unit.
    let mut net = NetworkRuntime::new(Some(&installation()));
    net.configure(
        plc("192.168.0.10"),
        &[scan(&GOOD.replace("\"unitId\": 1", "\"unitId\": 7"))],
    );
    run(&mut net, 50, 400);
    assert_eq!(net.scan_status(0), Some(ScanStatus::Ok));
    assert_eq!(net.value(0, 0), Some(ScanValue::Int(400)));

    // A gateway that serves only unit 1 rejects unit 7 with exception 0B.
    let text = serde_json::to_string(&installation()).unwrap().replace(
        r#""unitIds":{"type":"ignore"}"#,
        r#""unitIds":{"type":"accept","units":[1]}"#,
    );
    assert!(text.contains(r#""units":[1]"#), "{text}");
    let gateway: NetworkInstallation = serde_json::from_str(&text).unwrap();
    let mut net = NetworkRuntime::new(Some(&gateway));
    net.configure(
        plc("192.168.0.10"),
        &[scan(&GOOD.replace("\"unitId\": 1", "\"unitId\": 7"))],
    );
    run(&mut net, 50, 400);
    assert_eq!(
        net.scan_status(0),
        Some(ScanStatus::Exception(exception::GATEWAY_TARGET_FAILED))
    );
    assert_eq!(
        net.value(0, 0),
        None,
        "no value is invented after an exception"
    );
    let mut net = NetworkRuntime::new(Some(&gateway));
    net.configure(plc("192.168.0.10"), &[scan(GOOD)]);
    run(&mut net, 50, 400);
    assert_eq!(net.scan_status(0), Some(ScanStatus::Ok));
}

#[test]
fn word_order_mismatch_produces_a_wrong_but_explainable_value() {
    let mut net = NetworkRuntime::new(Some(&installation()));
    let swapped = GOOD.replace(
        r#"{"offset": 1, "dataType": {"type": "float32", "wordOrder": "highWordFirst"}"#,
        r#"{"offset": 1, "dataType": {"type": "float32", "wordOrder": "lowWordFirst"}"#,
    );
    net.configure(plc("192.168.0.10"), &[scan(&swapped)]);
    run(&mut net, 50, 625);
    let ScanValue::Real(v) = net.value(0, 1).unwrap() else {
        panic!("finite expected")
    };
    assert!(
        v != 62.5 && v.abs() < 1e-30,
        "62.5 with swapped words is a denormal, got {v}"
    );
}

#[test]
fn faults_link_down_offline_and_address_conflict() {
    let mut net = NetworkRuntime::new(Some(&installation()));
    net.configure(plc("192.168.0.10"), &[scan(GOOD)]);
    net.set_device_link(&Identifier::new("lt-gw").unwrap(), false);
    run(&mut net, 1000, 500);
    assert_eq!(net.scan_status(0), Some(ScanStatus::Timeout));
    net.set_device_link(&Identifier::new("lt-gw").unwrap(), true);
    run(&mut net, 2000, 500);
    assert_eq!(net.scan_status(0), Some(ScanStatus::Ok));
    net.set_device_ip(&Identifier::new("hmi").unwrap(), Some(ip("192.168.0.20")));
    assert_eq!(net.ping(ip("192.168.0.20")), PingResult::DuplicateReplies);
    run(&mut net, 4000, 500);
    let events = net.drain_events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, NetworkEventKind::AddressConflict { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| e.kind == NetworkEventKind::ConnectionRefused),
        "half the transactions reach the HMI"
    );
}

#[test]
fn identical_runs_produce_identical_event_streams() {
    let go = || {
        let mut net = NetworkRuntime::new(Some(&installation()));
        net.configure(plc("192.168.0.10"), &[scan(GOOD)]);
        run(&mut net, 5000, 333);
        net.drain_events()
    };
    assert_eq!(go(), go());
}
