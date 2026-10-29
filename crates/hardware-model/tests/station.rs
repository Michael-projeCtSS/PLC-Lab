use hardware_model::{
    ChannelValue, DiagnosticEvent, DiagnosticKind, FindingCategory, HardwareFault, Measurement,
    ModuleStatus, Quality, StationRuntime,
};
use lab_model::{
    hardware::{ChannelMode, ChannelRef, ModuleType, SlotRef, StationConfig},
    profile::{AnalogRepresentation, PlatformProfile},
    scene::{ComponentKind, OutputStyle, SceneDefinition, TransmitterWiring},
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
            {"position": {"rack": 0, "slot": 4}, "module": "ai4.ui"},
            {"position": {"rack": 0, "slot": 5}, "module": "ao2.ui"}
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
             "channels": [{"channel": 0, "mode": "current4To20mA", "diagnostics": true}]},
            {"position": {"rack": 0, "slot": 5}, "module": "ao2.ui", "startAddress": 80,
             "channels": [{"channel": 0, "mode": "current4To20mA"}, {"channel": 1, "mode": "voltage0To10V"}]}
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

#[test]
fn a_current_loop_needs_exactly_one_power_source() {
    // 2-wire transmitters are loop-powered by the input; 4-wire ones drive a passive input.
    for (wiring, loop_supply, expected_ua) in [
        (TransmitterWiring::TwoWire, true, 12_000),
        (TransmitterWiring::TwoWire, false, 0),
        (TransmitterWiring::FourWire, false, 12_000),
        (TransmitterWiring::FourWire, true, 0),
    ] {
        let mut s = scene();
        for c in &mut s.components {
            if let ComponentKind::LevelTransmitter { wiring: w, .. } = &mut c.kind {
                *w = wiring;
            }
        }
        let mut config = s.reference_station.clone();
        let ai = config
            .modules
            .iter_mut()
            .find(|m| m.module == ModuleType::Ai4)
            .unwrap();
        ai.channels[0].loop_supply = loop_supply;
        let runtime = SceneRuntime::new(&s, 1).unwrap();
        let mut station = StationRuntime::new(
            config,
            s.installation.clone(),
            profile(AnalogRepresentation::S7Nominal27648),
        )
        .unwrap();
        assert_eq!(
            station.measure(ch(4, 0), &runtime),
            Measurement::Microamps(expected_ua),
            "{wiring:?}, loop supply {loop_supply}"
        );
        let reading = station.sample_inputs(&runtime, WorldTime::ZERO)[&ch(4, 0)];
        assert_eq!(
            reading.quality == Quality::WireBreak,
            expected_ua == 0,
            "{wiring:?}, loop supply {loop_supply}: {reading:?}"
        );
    }
}

#[test]
fn analog_outputs_drive_their_terminals_only_when_they_can() {
    let (scene, mut station) = setup();
    let ao = |c| ch(5, c);
    let commands = BTreeMap::from([
        (ao(0), hardware_model::analog::AnalogRaw::Int(13_824)),
        (ao(1), hardware_model::analog::AnalogRaw::Int(27_648)),
    ]);
    station.drive_analog(&commands, true);
    assert_eq!(
        station.measure(ao(0), &scene),
        Measurement::Microamps(12_000)
    );
    assert_eq!(
        station.measure(ao(1), &scene),
        Measurement::Millivolts(10_000)
    );
    let r = station.reading(ao(0)).unwrap();
    assert_eq!(
        (r.value, r.quality),
        (ChannelValue::Int(13_824), Quality::Good)
    );
    // Overflow is clamped and reported.
    station.drive_analog(
        &BTreeMap::from([(ao(1), hardware_model::analog::AnalogRaw::Int(32_767))]),
        true,
    );
    assert_eq!(station.reading(ao(1)).unwrap().quality, Quality::Overflow);
    // In STOP an analog output drives nothing.
    station.drive_analog(&commands, false);
    assert_eq!(station.measure(ao(0), &scene), Measurement::Microamps(0));
    // Without field supply the channel cannot drive, and says so.
    station.apply_fault(
        HardwareFault::FieldPowerMissing(SlotRef { rack: 0, slot: 5 }),
        WorldTime::ZERO,
    );
    station.drive_analog(&commands, true);
    assert_eq!(station.measure(ao(0), &scene), Measurement::Microamps(0));
    assert_eq!(
        station.reading(ao(0)).unwrap().quality,
        Quality::NoFieldPower
    );
}

#[test]
fn module_electronics_power_and_field_power_are_different_conditions() {
    let (mut scene, mut station) = setup();
    let slot = SlotRef { rack: 0, slot: 3 };
    let commands = BTreeMap::from([(ch(3, 0), true)]);
    // Field supply missing: the module answers (electronics fine) but cannot switch.
    station.apply_fault(HardwareFault::FieldPowerMissing(slot), WorldTime::ZERO);
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    let view = station
        .module_views()
        .into_iter()
        .find(|v| v.position == slot)
        .unwrap();
    assert_eq!(view.status, ModuleStatus::NoFieldPower);
    assert_eq!(
        station.measure(ch(3, 0), &scene),
        Measurement::Millivolts(0)
    );
    station.clear_fault(&HardwareFault::FieldPowerMissing(slot), WorldTime::ZERO);
    // Electronics supply missing: the module is installed but does not answer at all.
    station.apply_fault(HardwareFault::ModulePowerMissing(slot), WorldTime::ZERO);
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    let view = station
        .module_views()
        .into_iter()
        .find(|v| v.position == slot)
        .unwrap();
    assert_eq!(view.status, ModuleStatus::NoModulePower);
    assert_eq!(view.installed, Some(ModuleType::Do8), "physically present");
    assert_eq!(view.status_led, hardware_model::LedState::Off);
    assert_eq!(
        station.measure(ch(3, 0), &scene),
        Measurement::Millivolts(0)
    );
    assert!(
        station.active_diagnostics().any(|(s, k)| *s
            == hardware_model::DiagnosticSource::Module(slot)
            && *k == DiagnosticKind::ModuleMissing),
        "the CPU sees no answer, as for a missing module"
    );
    // Inputs of other modules are unaffected.
    assert_eq!(
        bool_at(&mut station, &scene, ch(2, 1)),
        (true, Quality::Good)
    );
}

fn categories(station: &StationRuntime, scene: &SceneRuntime) -> Vec<FindingCategory> {
    station.compare(scene).iter().map(|f| f.category).collect()
}

#[test]
fn comparing_project_installation_and_drawing_finds_each_difference() {
    let (healthy_scene, healthy) = setup();
    let findings = healthy.compare(&healthy_scene);
    assert!(findings.is_empty(), "{findings:?}");
    let s = |slot| SlotRef { rack: 0, slot };
    let with = |fault: HardwareFault| {
        let (scene, mut station) = setup();
        station.apply_fault(fault, WorldTime::ZERO);
        (scene, station)
    };
    let (sc, st) = with(HardwareFault::ModuleMissing(s(2)));
    assert_eq!(categories(&st, &sc), [FindingCategory::ModuleMissing]);
    let (sc, st) = with(HardwareFault::WrongModule(s(4), ModuleType::Di8Sinking));
    let f = st.compare(&sc);
    assert_eq!(f[0].category, FindingCategory::WrongModuleType);
    assert_eq!(f[0].module_id.as_deref(), Some("ai-r0s4"));
    assert_eq!(
        f[0].detected.as_deref(),
        Some(ModuleType::Di8Sinking.generic_name())
    );
    // A module moved to a free slot is one finding, not a missing plus an unexpected one.
    let (sc, st) = with(HardwareFault::WrongSlot(s(2), s(7)));
    assert_eq!(categories(&st, &sc), [FindingCategory::WrongSlot]);
    let (sc, st) = with(HardwareFault::WrongModule(s(7), ModuleType::Do8));
    assert_eq!(categories(&st, &sc), [FindingCategory::UnexpectedModule]);
    let (sc, st) = with(HardwareFault::ModulePowerMissing(s(3)));
    assert_eq!(categories(&st, &sc), [FindingCategory::ModulePowerMissing]);
    let (sc, st) = with(HardwareFault::FieldPowerMissing(s(3)));
    assert_eq!(categories(&st, &sc), [FindingCategory::FieldPowerMissing]);

    // Configuration against the drawing: the transmitter is a 2-wire 4-20 mA device.
    let configured = |edit: &dyn Fn(&mut StationConfig)| {
        let scene_def = scene();
        let mut config = scene_def.reference_station.clone();
        edit(&mut config);
        let runtime = SceneRuntime::new(&scene_def, 1).unwrap();
        let station = StationRuntime::new(
            config,
            scene_def.installation.clone(),
            profile(AnalogRepresentation::S7Nominal27648),
        )
        .unwrap();
        (runtime, station)
    };
    let (sc, st) = configured(&|c| {
        let m = c
            .modules
            .iter_mut()
            .find(|m| m.module == ModuleType::Ai4)
            .unwrap();
        m.channels[0].mode = ChannelMode::Voltage0To10V;
    });
    assert_eq!(categories(&st, &sc), [FindingCategory::ChannelModeMismatch]);
    let (sc, st) = configured(&|c| {
        let m = c
            .modules
            .iter_mut()
            .find(|m| m.module == ModuleType::Ai4)
            .unwrap();
        m.channels[0].loop_supply = false;
    });
    assert_eq!(categories(&st, &sc), [FindingCategory::LoopSupplyMismatch]);
    let (sc, st) = configured(&|c| {
        let m = c
            .modules
            .iter_mut()
            .find(|m| m.module == ModuleType::Do8)
            .unwrap();
        m.start_address = 64;
        let ai = c
            .modules
            .iter_mut()
            .find(|m| m.module == ModuleType::Ao2)
            .unwrap();
        ai.start_address = 64;
    });
    assert_eq!(categories(&st, &sc), [FindingCategory::AddressOverlap]);
    let (sc, st) = configured(&|c| {
        // Validation already refuses duplicate tags; a tag on a disabled channel passes it.
        let ai = c
            .modules
            .iter_mut()
            .find(|m| m.module == ModuleType::Ai4)
            .unwrap();
        ai.channels.push(lab_model::hardware::ChannelConfig {
            channel: 1,
            mode: ChannelMode::Disabled,
            diagnostics: false,
            stop_behavior: Default::default(),
            loop_supply: true,
        });
        c.io_map.push(lab_model::hardware::IoMapping {
            channel: ch(4, 1),
            tag: lab_model::ids::TagName::new("Spare").unwrap(),
        });
    });
    assert_eq!(
        categories(&st, &sc),
        [FindingCategory::SignalBindingMismatch]
    );
}

/// A large station: 1 CPU, 8 DI16, 8 DO16, 4 AI4 and 4 AO2 (280 channels, each mapped).
fn large_station() -> (StationConfig, lab_model::hardware::InstalledHardware) {
    let mut configured = Vec::new();
    let mut installed = Vec::new();
    let mut io_map = Vec::new();
    let mut slot = 1u8;
    for (module, count, channels, bytes, base, mode) in [
        ("di16.24vdc.sink", 8, 16u8, 2u16, 0u16, "digital"),
        ("do16.24vdc", 8, 16, 2, 0, "digital"),
        ("ai4.ui", 4, 4, 8, 64, "current4To20mA"),
        ("ao2.ui", 4, 2, 4, 64, "current4To20mA"),
    ] {
        for n in 0..count {
            let channel_configs: Vec<_> = (0..channels)
                .map(|c| serde_json::json!({"channel": c, "mode": mode, "diagnostics": true}))
                .collect();
            configured.push(serde_json::json!({
                "position": {"rack": 0, "slot": slot}, "module": module,
                "startAddress": base + n * bytes, "channels": channel_configs,
            }));
            installed
                .push(serde_json::json!({"position": {"rack": 0, "slot": slot}, "module": module}));
            for c in 0..channels {
                io_map.push(serde_json::json!({
                    "channel": {"rack": 0, "slot": slot, "channel": c},
                    "tag": format!("S{slot}C{c}"),
                }));
            }
            slot += 1;
        }
    }
    let configured: StationConfig = serde_json::from_value(serde_json::json!({
        "name": "PLC_1", "cpu": "cpu.modular", "modules": configured, "ioMap": io_map,
    }))
    .unwrap();
    let installed = serde_json::from_value(serde_json::json!({
        "cpu": "cpu.modular", "modules": installed, "wiring": [],
    }))
    .unwrap();
    (configured, installed)
}

/// Run the large station for `scans` cycles; returns the final views and findings.
fn run_large_station(scans: u64) -> (Vec<hardware_model::ModuleView>, usize, usize) {
    let (configured, installed) = large_station();
    let mut scene = SceneRuntime::new(&scene(), 1).unwrap();
    let mut station = StationRuntime::new(
        configured.clone(),
        installed,
        profile(AnalogRepresentation::S7Nominal27648),
    )
    .unwrap();
    let outputs: Vec<ChannelRef> = configured
        .modules
        .iter()
        .filter(|m| m.module == ModuleType::Do16)
        .flat_map(|m| (0..16).map(move |c| ch(m.position.slot, c)))
        .collect();
    let analog: Vec<ChannelRef> = configured
        .modules
        .iter()
        .filter(|m| m.module == ModuleType::Ao2)
        .flat_map(|m| (0..2).map(move |c| ch(m.position.slot, c)))
        .collect();
    let mut findings = 0;
    for scan in 0..scans {
        let now = WorldTime::from_micros(scan * 10_000);
        let readings = station.sample_inputs(&scene, now).len();
        assert_eq!(readings, 8 * 16 + 4 * 4, "every input channel is sampled");
        let commands: BTreeMap<_, _> = outputs
            .iter()
            .enumerate()
            .map(|(i, c)| (*c, (scan + i as u64).is_multiple_of(3)))
            .collect();
        station.drive_outputs(&commands, true, &mut scene).unwrap();
        let setpoints: BTreeMap<_, _> = analog
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let raw = ((scan * 97 + i as u64 * 1_000) % 27_648) as i16;
                (*c, hardware_model::analog::AnalogRaw::Int(raw))
            })
            .collect();
        station.drive_analog(&setpoints, true);
        if scan.is_multiple_of(10) {
            findings = station.compare(&scene).len();
        }
    }
    let diagnostics = station.diagnostics().entries().count();
    (station.module_views(), findings, diagnostics)
}

#[test]
fn a_large_station_scans_within_budget_and_deterministically() {
    let started = std::time::Instant::now();
    let (views, findings, diagnostics) = run_large_station(2_000);
    let elapsed = started.elapsed();
    println!("large station: 2000 scans in {elapsed:?}, {diagnostics} diagnostic entries");
    assert_eq!(views.len(), 24);
    assert_eq!(views.iter().map(|v| v.channels.len()).sum::<usize>(), 280);
    assert!(
        views.iter().all(|v| v.status == ModuleStatus::Ok),
        "{views:?}"
    );
    // Healthy and consistent: no differences between project and installation.
    assert_eq!(findings, 0);
    // Unwired 4-20 mA inputs with diagnostics report wire breaks once, not every scan.
    assert!(diagnostics <= 2 * 16, "{diagnostics} diagnostic entries");
    // Debug builds on shared CI runners: a generous budget that still catches a
    // per-scan blow-up (quadratic work over the channels of every module).
    assert!(elapsed.as_secs() < 30, "2000 scans took {elapsed:?}");
    // Determinism: the same scans give the same station state.
    assert_eq!(run_large_station(2_000).0, views);
}
