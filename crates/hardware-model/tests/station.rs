use hardware_model::{
    ChannelValue, DiagnosticEvent, DiagnosticKind, HardwareFault, Measurement, ModuleStatus, Quality, StationRuntime,
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
    ChannelRef { rack: 0, slot, channel }
}
fn setup() -> (SceneRuntime, StationRuntime) {
    let s = scene();
    let runtime = SceneRuntime::new(&s, 1).unwrap();
    let station = StationRuntime::new(s.reference_station.clone(), s.installation.clone(), profile(AnalogRepresentation::S7Nominal27648)).unwrap();
    (runtime, station)
}
fn bool_at(station: &mut StationRuntime, scene: &SceneRuntime, c: ChannelRef) -> (bool, Quality) {
    let r = station.sample_inputs(scene, WorldTime::ZERO)[&c];
    let ChannelValue::Bool(v) = r.value else { panic!("bool expected") };
    (v, r.quality)
}

#[test]
fn healthy_station_reads_field_devices_and_analog_values() {
    let (mut scene, mut station) = setup();
    assert_eq!(bool_at(&mut station, &scene, ch(2, 0)), (false, Quality::Good));
    assert_eq!(bool_at(&mut station, &scene, ch(2, 1)), (true, Quality::Good), "NC stop conducts at rest");
    scene.press("start", true).unwrap();
    assert_eq!(bool_at(&mut station, &scene, ch(2, 0)), (true, Quality::Good));
    let ai = station.sample_inputs(&scene, WorldTime::ZERO)[&ch(4, 0)];
    assert_eq!(ai.value, ChannelValue::Int(13_824));
    assert_eq!(station.measure(ch(4, 0), &scene), Measurement::Microamps(12_000));
    let views = station.module_views();
    assert!(views.iter().all(|v| v.status == ModuleStatus::Ok));
    assert_eq!(views[2].channels[0].address, "%IX64.0".replace("X64.0", "W64"));
}

#[test]
fn missing_mismatched_and_unpowered_modules_are_diagnosed() {
    let (mut scene, mut station) = setup();
    scene.press("start", true).unwrap();
    station.apply_fault(HardwareFault::ModuleMissing(SlotRef { rack: 0, slot: 2 }), WorldTime::from_micros(5));
    assert_eq!(bool_at(&mut station, &scene, ch(2, 0)), (false, Quality::ModuleFault));
    assert_eq!(station.module_status(SlotRef { rack: 0, slot: 2 }), ModuleStatus::Missing);
    let last = station.diagnostics().entries().last().copied().unwrap();
    assert_eq!((last.kind, last.event), (DiagnosticKind::ModuleMissing, DiagnosticEvent::Incoming));
    station.clear_fault(&HardwareFault::ModuleMissing(SlotRef { rack: 0, slot: 2 }), WorldTime::from_micros(9));
    let last = station.diagnostics().entries().last().copied().unwrap();
    assert_eq!(last.event, DiagnosticEvent::Outgoing);
    assert_eq!(bool_at(&mut station, &scene, ch(2, 0)), (true, Quality::Good));

    station.apply_fault(HardwareFault::WrongModule(SlotRef { rack: 0, slot: 2 }, ModuleType::Do8), WorldTime::ZERO);
    assert_eq!(station.module_status(SlotRef { rack: 0, slot: 2 }), ModuleStatus::Mismatch(ModuleType::Do8));
    station.clear_fault(&HardwareFault::WrongModule(SlotRef { rack: 0, slot: 2 }, ModuleType::Do8), WorldTime::ZERO);

    station.apply_fault(HardwareFault::FieldPowerMissing(SlotRef { rack: 0, slot: 2 }), WorldTime::ZERO);
    assert_eq!(bool_at(&mut station, &scene, ch(2, 0)), (false, Quality::NoFieldPower));
    assert_eq!(station.measure(ch(2, 0), &scene), Measurement::Millivolts(0));
}

#[test]
fn wrong_slot_leaves_configured_slot_empty_and_module_unconfigured() {
    let (mut scene, mut station) = setup();
    scene.press("start", true).unwrap();
    station.apply_fault(
        HardwareFault::WrongSlot(SlotRef { rack: 0, slot: 2 }, SlotRef { rack: 0, slot: 6 }),
        WorldTime::ZERO,
    );
    assert_eq!(bool_at(&mut station, &scene, ch(2, 0)), (false, Quality::ModuleFault));
    assert_eq!(station.module_status(SlotRef { rack: 0, slot: 6 }), ModuleStatus::Unconfigured);
    // The terminal block moved with the module: a meter at slot 6 sees the start button.
    assert_eq!(station.measure(ch(6, 0), &scene), Measurement::Millivolts(24_000));
}

#[test]
fn wiring_and_device_style_faults() {
    let (mut scene, mut station) = setup();
    scene.press("start", true).unwrap();
    station.apply_fault(HardwareFault::SwappedWires(ch(2, 0), ch(2, 1)), WorldTime::ZERO);
    scene.press("stop", true).unwrap();
    // Channel 0 now sees the pressed NC stop button (open), channel 1 sees start (closed).
    assert_eq!(bool_at(&mut station, &scene, ch(2, 0)).0, false);
    assert_eq!(bool_at(&mut station, &scene, ch(2, 1)).0, true);
    station.clear_fault(&HardwareFault::SwappedWires(ch(2, 0), ch(2, 1)), WorldTime::ZERO);
    station.apply_fault(HardwareFault::OpenCircuit(ch(2, 0)), WorldTime::ZERO);
    assert_eq!(bool_at(&mut station, &scene, ch(2, 0)).0, false);
    station.apply_fault(HardwareFault::ShortToSupply(ch(2, 2)), WorldTime::ZERO);
    assert_eq!(bool_at(&mut station, &scene, ch(2, 2)).0, true);
    station.clear_fault(&HardwareFault::ShortToSupply(ch(2, 2)), WorldTime::ZERO);
    scene.set_presence("prox", true).unwrap();
    assert_eq!(bool_at(&mut station, &scene, ch(2, 2)).0, true);
    let prox = scene.index_of_str("prox").unwrap();
    scene.apply_fault(prox, ComponentFault::OutputStyle(OutputStyle::Npn)).unwrap();
    assert_eq!(bool_at(&mut station, &scene, ch(2, 2)).0, false, "NPN sensor on a sinking input never switches");
}

#[test]
fn analog_range_mismatch_and_wire_break() {
    let (scene, mut station) = setup();
    let mut config: StationConfig = station.configuration().clone();
    config.modules[2].channels[0].mode = ChannelMode::Current0To20mA;
    station.reconfigure(config, WorldTime::ZERO).unwrap();
    // 12 mA read as 0-20 mA: 16588 instead of 13824.
    assert_eq!(station.sample_inputs(&scene, WorldTime::ZERO)[&ch(4, 0)].value, ChannelValue::Int(16_589));
    let mut fixed = station.configuration().clone();
    fixed.modules[2].channels[0].mode = ChannelMode::Current4To20mA;
    station.reconfigure(fixed, WorldTime::ZERO).unwrap();
    station.apply_fault(HardwareFault::OpenCircuit(ch(4, 0)), WorldTime::ZERO);
    let r = station.sample_inputs(&scene, WorldTime::from_micros(100))[&ch(4, 0)];
    assert_eq!((r.value, r.quality), (ChannelValue::Int(32_767), Quality::WireBreak));
    assert!(station.diagnostics().entries().any(|e| e.kind == DiagnosticKind::WireBreak));
}

#[test]
fn outputs_energize_devices_and_stop_behavior_applies() {
    let (mut scene, mut station) = setup();
    let lamp = scene.index_of_str("lamp").unwrap();
    let mut commands = BTreeMap::new();
    commands.insert(ch(3, 0), true);
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    assert_eq!(scene.quantity(lamp, "lit"), Some(process_simulation::Quantity::Bool(true)));
    assert_eq!(station.measure(ch(3, 0), &scene), Measurement::Millivolts(24_000));
    station.drive_outputs(&commands, false, &mut scene).unwrap();
    assert_eq!(scene.quantity(lamp, "lit"), Some(process_simulation::Quantity::Bool(false)), "outputs off in STOP");
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    station.apply_fault(HardwareFault::FieldPowerMissing(SlotRef { rack: 0, slot: 3 }), WorldTime::ZERO);
    station.drive_outputs(&commands, true, &mut scene).unwrap();
    assert_eq!(scene.quantity(lamp, "lit"), Some(process_simulation::Quantity::Bool(false)));
}

#[test]
fn cpu_mismatch_is_rejected() {
    let s = scene();
    let mut config = s.reference_station.clone();
    config.cpu = ModuleType::CpuCompact;
    assert!(StationRuntime::new(config, s.installation.clone(), PlatformProfile::generic()).is_err());
}
