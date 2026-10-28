use hardware_model::{
    ChannelValue, DiagnosticEvent, DiagnosticKind, HardwareFault, Measurement, ModuleStatus,
    Quality, StationRuntime,
};
use lab_model::{
    hardware::{ChannelMode, ChannelRef, ModuleType, SlotRef, StationConfig},
    profile::{AnalogRepresentation, PlatformProfile},
    scene::{OutputStyle, SceneDefinition},
    validate_scene,
};
use process_simulation::{ComponentFault, SceneRuntime};
use simulation_core::WorldTime;
use std::collections::BTreeMap;

fn scene() -> SceneDefinition {
    let scene: SceneDefinition = serde_json::from_str(
        r#"{
        "format": "plclab-scene", "schemaVersion": 1, "id": "test.station", "revision": 1,
        "title": "Station", "summary": "Test",
        "components": [
          {"id": "start", "label": "Start", "kind": {"type": "pushButton", "contact": "normallyOpen"}},
          {"id": "stop", "label": "Stop", "kind": {"type": "pushButton", "contact": "normallyClosed"}},
          {"id": "prox", "label": "Prox", "kind": {"type": "presenceSensor", "output": "pnp", "contact": "normallyOpen"}},
          {"id": "lamp", "label": "Lamp", "kind": {"type": "pilotLamp", "color": "green"}},
          {"id": "tank", "label": "Tank", "kind": {"type": "tank", "capacityMl": 10000, "initialMl": 5000}},
          {"id": "lt", "label": "LT", "kind": {"type": "levelTransmitter", "tank": "tank", "rangeMinMl": 0, "rangeMaxMl": 10000, "signal": "current4To20mA"}}
        ],
        "installation": {"cpu": "cpu.modular", "modules": [
            {"position": {"rack": 0, "slot": 2}, "module": "di8.24vdc.sink"},
            {"position": {"rack": 0, "slot": 3}, "module": "do8.24vdc"},
            {"position": {"rack": 0, "slot": 4}, "module": "ai4.ui"}
          ], "wiring": [
            {"channel": {"rack": 0, "slot": 2, "channel": 0}, "component": "start", "port": "contact"},
            {"channel": {"rack": 0, "slot": 2, "channel": 1}, "component": "stop", "port": "contact"},
            {"channel": {"rack": 0, "slot": 2, "channel": 2}, "component": "prox", "port": "output"},
            {"channel": {"rack": 0, "slot": 3, "channel": 0}, "component": "lamp", "port": "coil"},
            {"channel": {"rack": 0, "slot": 4, "channel": 0}, "component": "lt", "port": "signal"}
          ]},
        "referenceStation": {"name": "PLC_1", "cpu": "cpu.modular", "modules": [
            {"position": {"rack": 0, "slot": 2}, "module": "di8.24vdc.sink", "startAddress": 0},
            {"position": {"rack": 0, "slot": 3}, "module": "do8.24vdc", "startAddress": 0},
            {"position": {"rack": 0, "slot": 4}, "module": "ai4.ui", "startAddress": 64,
             "channels": [{"channel": 0, "mode": "current4To20mA", "diagnostics": true}]}
          ], "ioMap": [
            {"channel": {"rack": 0, "slot": 2, "channel": 0}, "tag": "StartPB"},
            {"channel": {"rack": 0, "slot": 3, "channel": 0}, "tag": "Lamp"},
            {"channel": {"rack": 0, "slot": 4, "channel": 0}, "tag": "LevelRaw"}
          ]}
      }"#,
    )
    .unwrap();
    validate_scene(&scene).unwrap();
    scene
}

fn profile(analog: AnalogRepresentation) -> PlatformProfile {
    let mut p = PlatformProfile::generic();
    p.analog = analog;
    p
}
fn ch(slot: u8, channel: u8) -> ChannelRef {
    ChannelRef {
        rack: 0,
        slot,
        channel,
    }
}
fn setup() -> (SceneRuntime, StationRuntime) {
    let s = scene();
    let runtime = SceneRuntime::new(&s, 1).unwrap();
    let station = StationRuntime::new(
        s.reference_station.clone(),
        s.installation.clone(),
        profile(AnalogRepresentation::S7Nominal27648),
    )
    .unwrap();
    (runtime, station)
}
fn bool_at(station: &mut StationRuntime, scene: &SceneRuntime, c: ChannelRef) -> (bool, Quality) {
    let r = station.sample_inputs(scene, WorldTime::ZERO)[&c];
    let ChannelValue::Bool(v) = r.value else {
        panic!("bool expected")
    };
    (v, r.quality)
}

#[test]
fn healthy_station_reads_field_devices_and_analog_values() {
    let (mut scene, mut station) = setup();
    assert_eq!(
        bool_at(&mut station, &scene, ch(2, 0)),
        (false, Quality::Good)
    );
    assert_eq!(
        bool_at(&mut station, &scene, ch(2, 1)),
        (true, Quality::Good),
        "NC stop conducts at rest"
    );
    scene.press("start", true).unwrap();
    assert_eq!(
        bool_at(&mut station, &scene, ch(2, 0)),
        (true, Quality::Good)
    );
    let ai = station.sample_inputs(&scene, WorldTime::ZERO)[&ch(4, 0)];
    assert_eq!(ai.value, ChannelValue::Int(13_824));
    assert_eq!(
        station.measure(ch(4, 0), &scene),
        Measurement::Microamps(12_000)
    );
    let views = station.module_views();
    assert!(views.iter().all(|v| v.status == ModuleStatus::Ok));
    assert_eq!(
        views[2].channels[0].address,
        "%IX64.0".replace("X64.0", "W64")
    );
}

#[test]
fn missing_mismatched_and_unpowered_modules_are_diagnosed() {
    let (mut scene, mut station) = setup();
    scene.press("start", true).unwrap();
    station.apply_fault(
        HardwareFault::ModuleMissing(SlotRef { rack: 0, slot: 2 }),
        WorldTime::from_micros(5),
    );
    assert_eq!(
        bool_at(&mut station, &scene, ch(2, 0)),
        (false, Quality::ModuleFault)
    );
    assert_eq!(
        station.module_status(SlotRef { rack: 0, slot: 2 }),
        ModuleStatus::Missing
    );
    let last = station.diagnostics().entries().last().copied().unwrap();
    assert_eq!(
        (last.kind, last.event),
        (DiagnosticKind::ModuleMissing, DiagnosticEvent::Incoming)
    );
    station.clear_fault(
        &HardwareFault::ModuleMissing(SlotRef { rack: 0, slot: 2 }),
        WorldTime::from_micros(9),
    );
    let last = station.diagnostics().entries().last().copied().unwrap();
    assert_eq!(last.event, DiagnosticEvent::Outgoing);
    assert_eq!(
        bool_at(&mut station, &scene, ch(2, 0)),
        (true, Quality::Good)
    );

    station.apply_fault(
        HardwareFault::WrongModule(SlotRef { rack: 0, slot: 2 }, ModuleType::Do8),
        WorldTime::ZERO,
    );
    assert_eq!(
        station.module_status(SlotRef { rack: 0, slot: 2 }),
        ModuleStatus::Mismatch(ModuleType::Do8)
    );
    station.clear_fault(
        &HardwareFault::WrongModule(SlotRef { rack: 0, slot: 2 }, ModuleType::Do8),
        WorldTime::ZERO,
    );

    station.apply_fault(
        HardwareFault::FieldPowerMissing(SlotRef { rack: 0, slot: 2 }),
        WorldTime::ZERO,
    );
    assert_eq!(
        bool_at(&mut station, &scene, ch(2, 0)),
        (false, Quality::NoFieldPower)
    );
    assert_eq!(
        station.measure(ch(2, 0), &scene),
        Measurement::Millivolts(0)
    );
}

#[test]
fn wrong_slot_leaves_configured_slot_empty_and_module_unconfigured() {
    let (mut scene, mut station) = setup();
    scene.press("start", true).unwrap();
    station.apply_fault(
        HardwareFault::WrongSlot(SlotRef { rack: 0, slot: 2 }, SlotRef { rack: 0, slot: 6 }),
        WorldTime::ZERO,
    );
    assert_eq!(
        bool_at(&mut station, &scene, ch(2, 0)),
        (false, Quality::ModuleFault)
    );
    assert_eq!(
        station.module_status(SlotRef { rack: 0, slot: 6 }),
        ModuleStatus::Unconfigured
    );
    // The terminal block moved with the module: a meter at slot 6 sees the start button.
    assert_eq!(
        station.measure(ch(6, 0), &scene),
        Measurement::Millivolts(24_000)
    );
}

#[test]
fn wiring_and_device_style_faults() {
    let (mut scene, mut station) = setup();
    scene.press("start", true).unwrap();
    station.apply_fault(
        HardwareFault::SwappedWires(ch(2, 0), ch(2, 1)),
        WorldTime::ZERO,
    );
    scene.press("stop", true).unwrap();
    // Channel 0 now sees the pressed NC stop button (open), channel 1 sees start (closed).
    assert!(!bool_at(&mut station, &scene, ch(2, 0)).0);
    assert!(bool_at(&mut station, &scene, ch(2, 1)).0);
    station.clear_fault(
        &HardwareFault::SwappedWires(ch(2, 0), ch(2, 1)),
        WorldTime::ZERO,
    );
    station.apply_fault(HardwareFault::OpenCircuit(ch(2, 0)), WorldTime::ZERO);
    assert!(!bool_at(&mut station, &scene, ch(2, 0)).0);
    station.apply_fault(HardwareFault::ShortToSupply(ch(2, 2)), WorldTime::ZERO);
    assert!(bool_at(&mut station, &scene, ch(2, 2)).0);
    station.clear_fault(&HardwareFault::ShortToSupply(ch(2, 2)), WorldTime::ZERO);
    scene.set_presence("prox", true).unwrap();
    assert!(bool_at(&mut station, &scene, ch(2, 2)).0);
    let prox = scene.index_of_str("prox").unwrap();
    scene
        .apply_fault(prox, ComponentFault::OutputStyle(OutputStyle::Npn))
        .unwrap();
    assert!(
        !bool_at(&mut station, &scene, ch(2, 2)).0,
        "NPN sensor on a sinking input never switches"
    );
}

#[test]
fn analog_range_mismatch_and_wire_break() {
    let (scene, mut station) = setup();
    let mut config: StationConfig = station.configuration().clone();
    config.modules[2].channels[0].mode = ChannelMode::Current0To20mA;
    station.reconfigure(config, WorldTime::ZERO).unwrap();
    // 12 mA read as 0-20 mA: 16588 instead of 13824.
    assert_eq!(
        station.sample_inputs(&scene, WorldTime::ZERO)[&ch(4, 0)].value,
        ChannelValue::Int(16_589)
    );
    let mut fixed = station.configuration().clone();
    fixed.modules[2].channels[0].mode = ChannelMode::Current4To20mA;
    station.reconfigure(fixed, WorldTime::ZERO).unwrap();
    station.apply_fault(HardwareFault::OpenCircuit(ch(4, 0)), WorldTime::ZERO);
    let r = station.sample_inputs(&scene, WorldTime::from_micros(100))[&ch(4, 0)];
    assert_eq!(
        (r.value, r.quality),
        (ChannelValue::Int(32_767), Quality::WireBreak)
    );
    assert!(
        station
            .diagnostics()
            .entries()
            .any(|e| e.kind == DiagnosticKind::WireBreak)
    );
}

#[test]
fn outputs_energize_devices_and_stop_behavior_applies() {
    let (mut scene, mut station) = setup();
    let lamp = scene.index_of_str("lamp").unwrap();
    let mut commands = BTreeMap::new();
    commands.insert(ch(3, 0), true);
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    assert_eq!(
        scene.quantity(lamp, "lit"),
        Some(process_simulation::Quantity::Bool(true))
    );
    assert_eq!(
        station.measure(ch(3, 0), &scene),
        Measurement::Millivolts(24_000)
    );
    station.drive_outputs(&commands, false, &mut scene).unwrap();
    assert_eq!(
        scene.quantity(lamp, "lit"),
        Some(process_simulation::Quantity::Bool(false)),
        "outputs off in STOP"
    );
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    station.apply_fault(
        HardwareFault::FieldPowerMissing(SlotRef { rack: 0, slot: 3 }),
        WorldTime::ZERO,
    );
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    assert_eq!(
        scene.quantity(lamp, "lit"),
        Some(process_simulation::Quantity::Bool(false))
    );
}

#[test]
fn cpu_mismatch_is_rejected() {
    let s = scene();
    let mut config = s.reference_station.clone();
    config.cpu = ModuleType::CpuCompact;
    assert!(
        StationRuntime::new(config, s.installation.clone(), PlatformProfile::generic()).is_err()
    );
}

const ACTUATORS: &str = r#"{
  "format": "plclab-scene", "schemaVersion": 1, "id": "test.actuators", "revision": 1,
  "title": "Actuators", "summary": "Every digital sink type on one output module",
  "components": [
    {"id": "valve", "label": "Valve", "kind": {"type": "onOffValve", "travelMs": 500, "flowMlPerS": 100}},
    {"id": "starter", "label": "Starter", "kind": {"type": "motorStarter", "pickupMs": 30, "runUpMs": 500, "ratedCurrentMa": 2000}},
    {"id": "drive", "label": "Drive", "kind": {"type": "variableFrequencyDrive", "rampMs": 1000, "ratedCurrentMa": 3000}},
    {"id": "pump", "label": "Pump", "kind": {"type": "pump", "flowMlPerS": 50}},
    {"id": "prox", "label": "Prox", "kind": {"type": "presenceSensor", "output": "pnp", "contact": "normallyOpen"}}
  ],
  "installation": {"cpu": "cpu.modular", "modules": [
      {"position": {"rack": 0, "slot": 2}, "module": "di8.24vdc.sink"},
      {"position": {"rack": 0, "slot": 3}, "module": "do8.24vdc"}
    ], "wiring": [
      {"channel": {"rack": 0, "slot": 3, "channel": 0}, "component": "valve", "port": "solenoid"},
      {"channel": {"rack": 0, "slot": 3, "channel": 1}, "component": "starter", "port": "coil"},
      {"channel": {"rack": 0, "slot": 3, "channel": 2}, "component": "drive", "port": "run"},
      {"channel": {"rack": 0, "slot": 3, "channel": 3}, "component": "pump", "port": "motor"},
      {"channel": {"rack": 0, "slot": 2, "channel": 0}, "component": "starter", "port": "aux"},
      {"channel": {"rack": 0, "slot": 2, "channel": 1}, "component": "drive", "port": "running"},
      {"channel": {"rack": 0, "slot": 2, "channel": 2}, "component": "prox", "port": "output"}
    ]},
  "referenceStation": {"name": "PLC_1", "cpu": "cpu.modular", "modules": [
      {"position": {"rack": 0, "slot": 2}, "module": "di8.24vdc.sink", "startAddress": 0},
      {"position": {"rack": 0, "slot": 3}, "module": "do8.24vdc", "startAddress": 0}
    ], "ioMap": []}
}"#;

fn command(scene: &SceneRuntime, component: &str) -> bool {
    let index = scene.index_of_str(component).unwrap();
    scene.quantity(index, "command") == Some(process_simulation::Quantity::Bool(true))
}

#[test]
fn output_energization_follows_port_direction_not_port_names() {
    let definition: SceneDefinition = serde_json::from_str(ACTUATORS).unwrap();
    validate_scene(&definition).unwrap();
    let mut scene = SceneRuntime::new(&definition, 1).unwrap();
    let mut station = StationRuntime::new(
        definition.reference_station.clone(),
        definition.installation.clone(),
        PlatformProfile::generic(),
    )
    .unwrap();
    // Each kind declares its sinks; names such as "output" or "running" do not make a
    // port an output.
    for (component, port, sink) in [
        ("valve", "solenoid", true),
        ("valve", "openSwitch", false),
        ("starter", "coil", true),
        ("starter", "aux", false),
        ("starter", "overload", false),
        ("drive", "run", true),
        ("drive", "running", false),
        ("drive", "remote", false),
        ("pump", "motor", true),
        ("prox", "output", false),
    ] {
        let index = scene.index_of_str(component).unwrap();
        assert_eq!(
            scene.is_digital_sink(index, port),
            sink,
            "{component}.{port}"
        );
    }
    let all = ["valve", "starter", "drive", "pump"];
    let mut commands = BTreeMap::new();
    for channel in 0..4 {
        commands.insert(ch(3, channel), true);
    }
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    for c in all {
        assert!(command(&scene, c), "{c} is energized by its output channel");
    }
    // Commands on input channels never energize anything.
    let mut inputs = BTreeMap::new();
    for channel in 0..3 {
        inputs.insert(ch(2, channel), true);
    }
    station.drive_outputs(&inputs, true, &mut scene).unwrap();
    for c in all {
        assert!(
            !command(&scene, c),
            "{c} is de-energized without its output"
        );
    }
    // One channel switches exactly its own device.
    station
        .drive_outputs(&BTreeMap::from([(ch(3, 2), true)]), true, &mut scene)
        .unwrap();
    assert!(command(&scene, "drive"));
    for c in ["valve", "starter", "pump"] {
        assert!(!command(&scene, c), "{c}");
    }
}

#[test]
fn wiring_against_port_direction_or_to_unknown_ports_is_rejected() {
    let base: serde_json::Value = serde_json::from_str(ACTUATORS).unwrap();
    let rewire = |channel: serde_json::Value, component: &str, port: &str| {
        let mut scene = base.clone();
        scene["installation"]["wiring"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"channel": channel, "component": component, "port": port}));
        serde_json::from_value::<SceneDefinition>(scene).unwrap()
    };
    let output = serde_json::json!({"rack": 0, "slot": 3, "channel": 7});
    let input = serde_json::json!({"rack": 0, "slot": 2, "channel": 7});
    for (channel, component, port) in [
        (output.clone(), "prox", "output"),
        (output.clone(), "drive", "ready"),
        (output.clone(), "starter", "aux"),
        (input.clone(), "drive", "run"),
        (input.clone(), "valve", "solenoid"),
        (output.clone(), "drive", "speed"),
        (output.clone(), "nothing", "coil"),
    ] {
        let scene = rewire(channel, component, port);
        assert!(
            validate_scene(&scene).is_err(),
            "{component}.{port} must be rejected"
        );
    }
    // Unknown component types and unknown parameters are rejected while parsing.
    let mut unknown = base.clone();
    unknown["components"][2]["kind"]["type"] = serde_json::json!("frequencyDrive");
    assert!(serde_json::from_value::<SceneDefinition>(unknown).is_err());
    let mut extra = base.clone();
    extra["components"][2]["kind"]["direction"] = serde_json::json!("output");
    assert!(serde_json::from_value::<SceneDefinition>(extra).is_err());
}
