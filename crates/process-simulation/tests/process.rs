use lab_model::{
    fault::StuckPosition,
    scene::{ComponentKind, ContactType, OutputStyle, SceneDefinition},
    validate_scene,
};
use process_simulation::{AnalogUnit, ComponentFault, ProcessEventKind, Quantity, SceneRuntime};
use simulation_core::WorldTime;

fn scene() -> SceneDefinition {
    let scene: SceneDefinition = serde_json::from_str(
        r#"{
        "format": "plclab-scene", "schemaVersion": 1, "id": "test.tank", "revision": 1,
        "title": "Test tank", "summary": "Test",
        "components": [
          {"id": "start", "label": "Start", "kind": {"type": "pushButton", "contact": "normallyOpen"}},
          {"id": "stop", "label": "Stop", "kind": {"type": "pushButton", "contact": "normallyClosed"}},
          {"id": "inlet", "label": "Inlet", "kind": {"type": "onOffValve", "travelMs": 2000, "flowMlPerS": 1000}},
          {"id": "pump", "label": "Pump", "kind": {"type": "pump", "flowMlPerS": 800}},
          {"id": "tank", "label": "Tank", "kind": {"type": "tank", "capacityMl": 10000, "initialMl": 0, "highHighMl": 9500, "lowLowMl": 500}},
          {"id": "high", "label": "High", "kind": {"type": "levelSwitch", "tank": "tank", "switchLevelMl": 8000, "contact": "normallyOpen", "output": "pnp"}},
          {"id": "lt", "label": "LT", "kind": {"type": "levelTransmitter", "tank": "tank", "rangeMinMl": 0, "rangeMaxMl": 10000, "signal": "current4To20mA"}},
          {"id": "motor", "label": "Motor", "kind": {"type": "motorStarter", "pickupMs": 30, "runUpMs": 2000, "ratedCurrentMa": 4200}},
          {"id": "drive", "label": "Drive", "kind": {"type": "variableFrequencyDrive", "rampMs": 2000, "ratedCurrentMa": 5000}}
        ],
        "couplings": [
          {"type": "valveFillsTank", "valve": "inlet", "tank": "tank"},
          {"type": "pumpDrainsTank", "pump": "pump", "tank": "tank"}
        ],
        "installation": {"cpu": "cpu.compact", "modules": [], "wiring": []},
        "referenceStation": {"name": "S", "cpu": "cpu.compact", "modules": [], "ioMap": []}
      }"#,
    )
    .unwrap();
    validate_scene(&scene).unwrap();
    scene
}

fn q(s: &SceneRuntime, component: &str, name: &str) -> Quantity {
    s.quantity(s.index_of_str(component).unwrap(), name)
        .unwrap()
}
fn at(ms: u64) -> WorldTime {
    WorldTime::from_micros(ms * 1000)
}

#[test]
fn valve_travel_gates_flow_at_half_travel_and_fills_exactly() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    let inlet = s.index_of_str("inlet").unwrap();
    s.set_coil(inlet, "solenoid", true).unwrap();
    s.advance_to(at(999)).unwrap();
    assert_eq!(q(&s, "tank", "levelMl"), Quantity::Integer(0));
    assert_eq!(q(&s, "inlet", "positionPermille"), Quantity::Integer(499));
    s.advance_to(at(3000)).unwrap();
    // Flow from 1.0 s to 3.0 s at 1000 ml/s.
    assert_eq!(q(&s, "tank", "levelMl"), Quantity::Integer(2000));
    assert_eq!(q(&s, "inlet", "positionPermille"), Quantity::Integer(1000));
    let open = s.digital_output(inlet, "openSwitch").unwrap();
    assert!(open.conducting);
}

#[test]
fn partitioning_world_time_never_changes_physics() {
    let run = |steps: &[u64]| {
        let mut s = SceneRuntime::new(&scene(), 7).unwrap();
        let inlet = s.index_of_str("inlet").unwrap();
        let pump = s.index_of_str("pump").unwrap();
        s.set_coil(inlet, "solenoid", true).unwrap();
        let mut t = 0;
        for (i, step) in steps.iter().cycle().take(4000).enumerate() {
            t += step;
            s.advance_to(WorldTime::from_micros(t)).unwrap();
            if i == 50 {
                s.set_coil(pump, "motor", true).unwrap();
            }
        }
        s.advance_to(WorldTime::from_micros(20_000_000)).unwrap();
        (q(&s, "tank", "levelMl"), q(&s, "tank", "spilledMl"))
    };
    let reference = run(&[100]);
    assert_eq!(run(&[100]), reference);
    // Different partitions only shift the moment of the pump command, so compare runs
    // that place it at the same time: 50 steps of 100 us == 25 steps of 200 us is not equal;
    // instead check that pure integration (no commands) is partition-invariant.
    let pure = |step: u64| {
        let mut s = SceneRuntime::new(&scene(), 7).unwrap();
        let inlet = s.index_of_str("inlet").unwrap();
        s.set_coil(inlet, "solenoid", true).unwrap();
        let mut t = 0;
        while t < 7_333_333 {
            t = (t + step).min(7_333_333);
            s.advance_to(WorldTime::from_micros(t)).unwrap();
        }
        q(&s, "tank", "levelMl")
    };
    let expected = pure(7_333_333);
    for step in [1_000, 3_333, 100_000, 999_999] {
        assert_eq!(pure(step), expected, "step {step}");
    }
}

#[test]
fn overflow_high_high_and_dry_run_events_have_exact_times() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    let inlet = s.index_of_str("inlet").unwrap();
    s.set_coil(inlet, "solenoid", true).unwrap();
    s.advance_to(at(12_000)).unwrap();
    let events = s.drain_events();
    let hh = events
        .iter()
        .find(|e| e.kind == ProcessEventKind::HighHighOn)
        .unwrap();
    // 9500 ml at 1000 ml/s after flow starts at 1.0 s.
    assert_eq!(hh.time, at(10_500));
    let of = events
        .iter()
        .find(|e| e.kind == ProcessEventKind::OverflowStarted)
        .unwrap();
    assert_eq!(of.time, at(11_000));
    assert_eq!(q(&s, "tank", "spilledMl"), Quantity::Integer(1000));
    assert_eq!(q(&s, "tank", "overflowing"), Quantity::Bool(true));

    s.set_coil(inlet, "solenoid", false).unwrap();
    let pump = s.index_of_str("pump").unwrap();
    s.set_coil(pump, "motor", true).unwrap();
    s.advance_to(at(30_000)).unwrap();
    let events = s.drain_events();
    assert!(
        events
            .iter()
            .any(|e| e.kind == ProcessEventKind::OverflowEnded)
    );
    assert!(events.iter().any(|e| e.kind == ProcessEventKind::LowLowOn));
    let dry = events
        .iter()
        .find(|e| e.kind == ProcessEventKind::DryRunStarted)
        .unwrap();
    assert!(dry.time > at(12_000) && dry.time <= at(30_000));
    assert_eq!(q(&s, "pump", "dry"), Quantity::Bool(true));
    assert_eq!(q(&s, "tank", "levelMl"), Quantity::Integer(0));
}

#[test]
fn contacts_follow_no_nc_and_faults_override_them() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    let start = s.index_of_str("start").unwrap();
    let stop = s.index_of_str("stop").unwrap();
    assert!(!s.digital_output(start, "contact").unwrap().conducting);
    assert!(
        s.digital_output(stop, "contact").unwrap().conducting,
        "NC stop button conducts at rest"
    );
    s.press("start", true).unwrap();
    s.press("stop", true).unwrap();
    assert!(s.digital_output(start, "contact").unwrap().conducting);
    assert!(!s.digital_output(stop, "contact").unwrap().conducting);
    s.apply_fault(
        start,
        ComponentFault::ContactType(ContactType::NormallyClosed),
    )
    .unwrap();
    assert!(!s.digital_output(start, "contact").unwrap().conducting);
    s.apply_fault(start, ComponentFault::SensorStuck(true))
        .unwrap();
    s.press("start", false).unwrap();
    assert!(s.digital_output(start, "contact").unwrap().conducting);
    s.clear_fault(start, &ComponentFault::SensorStuck(true))
        .unwrap();
    s.clear_fault(
        start,
        &ComponentFault::ContactType(ContactType::NormallyClosed),
    )
    .unwrap();
    assert!(!s.digital_output(start, "contact").unwrap().conducting);
    let high = s.index_of_str("high").unwrap();
    s.apply_fault(high, ComponentFault::OutputStyle(OutputStyle::Npn))
        .unwrap();
    assert_eq!(
        s.digital_output(high, "contact").unwrap().style,
        OutputStyle::Npn
    );
    assert!(s.press("tank", true).is_err());
}

#[test]
fn transmitter_is_linear_saturates_and_reports_faults() {
    let mut s = SceneRuntime::new(&scene(), 99).unwrap();
    let lt = s.index_of_str("lt").unwrap();
    let a = s.analog_output(lt, "signal").unwrap();
    assert_eq!((a.unit, a.value), (AnalogUnit::Microamp, 4_000));
    let inlet = s.index_of_str("inlet").unwrap();
    s.set_coil(inlet, "solenoid", true).unwrap();
    s.advance_to(at(6_000)).unwrap(); // 5000 ml = 50 %
    assert_eq!(s.analog_output(lt, "signal").unwrap().value, 12_000);
    s.advance_to(at(20_000)).unwrap(); // full and overflowing
    assert_eq!(s.analog_output(lt, "signal").unwrap().value, 20_000);
    s.apply_fault(lt, ComponentFault::AnalogDrift(1_000))
        .unwrap();
    assert_eq!(s.analog_output(lt, "signal").unwrap().value, 21_000);
    s.apply_fault(lt, ComponentFault::LoopBreak).unwrap();
    assert_eq!(s.analog_output(lt, "signal").unwrap().value, 0);
    s.clear_fault(lt, &ComponentFault::LoopBreak).unwrap();
    s.clear_fault(lt, &ComponentFault::AnalogDrift(0)).unwrap();
    s.apply_fault(lt, ComponentFault::AnalogNoise(50)).unwrap();
    let v1 = s.analog_output(lt, "signal").unwrap().value;
    assert!((19_950..=20_050).contains(&v1));
    assert_eq!(
        v1,
        s.analog_output(lt, "signal").unwrap().value,
        "noise is deterministic"
    );
}

#[test]
fn motor_starter_picks_up_runs_up_and_trips() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    let motor = s.index_of_str("motor").unwrap();
    s.set_coil(motor, "coil", true).unwrap();
    s.advance_to(at(29)).unwrap();
    assert_eq!(q(&s, "motor", "running"), Quantity::Bool(false));
    s.advance_to(at(30)).unwrap();
    assert!(s.digital_output(motor, "aux").unwrap().conducting);
    s.advance_to(at(1_030)).unwrap();
    assert_eq!(q(&s, "motor", "speedPermille"), Quantity::Integer(500));
    s.apply_fault(motor, ComponentFault::MotorOverload).unwrap();
    assert_eq!(q(&s, "motor", "running"), Quantity::Bool(false));
    assert!(!s.digital_output(motor, "overload").unwrap().conducting);
    assert!(
        s.drain_events()
            .iter()
            .any(|e| e.kind == ProcessEventKind::MotorOverloadTripped)
    );
    s.clear_fault(motor, &ComponentFault::MotorOverload)
        .unwrap();
    s.advance_to(at(1_060)).unwrap();
    assert_eq!(q(&s, "motor", "running"), Quantity::Bool(true));
}

#[test]
fn stuck_valve_ignores_commands_and_resumes_after_repair() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    let inlet = s.index_of_str("inlet").unwrap();
    s.apply_fault(inlet, ComponentFault::ValveStuck(StuckPosition::Closed))
        .unwrap();
    s.set_coil(inlet, "solenoid", true).unwrap();
    s.advance_to(at(5_000)).unwrap();
    assert_eq!(q(&s, "tank", "levelMl"), Quantity::Integer(0));
    assert!(s.digital_output(inlet, "closedSwitch").unwrap().conducting);
    s.clear_fault(inlet, &ComponentFault::ValveStuck(StuckPosition::Closed))
        .unwrap();
    s.advance_to(at(8_000)).unwrap();
    assert_eq!(q(&s, "tank", "levelMl"), Quantity::Integer(2000));
    assert!(s.advance_to(at(1)).is_err());
}

#[test]
fn drive_ramps_on_its_run_terminal_and_ignores_it_in_local_control() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    let drive = s.index_of_str("drive").unwrap();
    let relay = |s: &SceneRuntime, port: &str| s.digital_output(drive, port).unwrap().conducting;
    assert!(relay(&s, "ready") && relay(&s, "remote") && !relay(&s, "running"));
    s.set_coil(drive, "run", true).unwrap();
    s.advance_to(at(1000)).unwrap();
    assert!(relay(&s, "running"));
    assert_eq!(q(&s, "drive", "speedPermille"), Quantity::Integer(500));
    s.advance_to(at(3000)).unwrap();
    assert_eq!(q(&s, "drive", "speedPermille"), Quantity::Integer(1000));
    // Switching to local control stops the drive although the run command stays on.
    s.apply_fault(drive, ComponentFault::DriveLocalControl)
        .unwrap();
    assert!(!relay(&s, "running") && !relay(&s, "remote") && relay(&s, "ready"));
    assert_eq!(q(&s, "drive", "command"), Quantity::Bool(true));
    s.set_coil(drive, "run", false).unwrap();
    s.set_coil(drive, "run", true).unwrap();
    s.advance_to(at(4000)).unwrap();
    assert_eq!(q(&s, "drive", "running"), Quantity::Bool(false));
    // Back to remote: the pending run command starts the drive again.
    s.clear_fault(drive, &ComponentFault::DriveLocalControl)
        .unwrap();
    s.advance_to(at(5000)).unwrap();
    assert_eq!(q(&s, "drive", "speedPermille"), Quantity::Integer(500));
    s.set_coil(drive, "run", false).unwrap();
    assert_eq!(q(&s, "drive", "running"), Quantity::Bool(false));
}

#[test]
fn commanded_quantities_come_from_sink_port_direction() {
    // Every component kind of this scene plus a tank for the level devices.
    let scene: SceneDefinition = serde_json::from_str(
        r#"{
      "format": "plclab-scene", "schemaVersion": 1, "id": "test.kinds", "revision": 1,
      "title": "Kinds", "summary": "One of each",
      "components": [
        {"id": "pb", "label": "PB", "kind": {"type": "pushButton", "contact": "normallyOpen"}},
        {"id": "sel", "label": "Sel", "kind": {"type": "selectorSwitch", "positions": ["A", "B"]}},
        {"id": "lamp", "label": "Lamp", "kind": {"type": "pilotLamp", "color": "green"}},
        {"id": "starter", "label": "Starter", "kind": {"type": "motorStarter", "pickupMs": 30, "runUpMs": 500, "ratedCurrentMa": 2000}},
        {"id": "valve", "label": "Valve", "kind": {"type": "onOffValve", "travelMs": 500, "flowMlPerS": 100}},
        {"id": "pump", "label": "Pump", "kind": {"type": "pump", "flowMlPerS": 50}},
        {"id": "tank", "label": "Tank", "kind": {"type": "tank", "capacityMl": 1000, "initialMl": 100}},
        {"id": "ls", "label": "LS", "kind": {"type": "levelSwitch", "tank": "tank", "switchLevelMl": 500, "contact": "normallyOpen", "output": "pnp"}},
        {"id": "lt", "label": "LT", "kind": {"type": "levelTransmitter", "tank": "tank", "rangeMinMl": 0, "rangeMaxMl": 1000, "signal": "current4To20mA"}},
        {"id": "prox", "label": "Prox", "kind": {"type": "presenceSensor", "output": "pnp", "contact": "normallyOpen"}},
        {"id": "drive", "label": "Drive", "kind": {"type": "variableFrequencyDrive", "rampMs": 1000, "ratedCurrentMa": 3000}}
      ],
      "installation": {"cpu": "cpu.compact", "modules": [], "wiring": []},
      "referenceStation": {"name": "PLC_1", "cpu": "cpu.compact", "modules": [], "ioMap": []}
    }"#,
    )
    .unwrap();
    let runtime = SceneRuntime::new(&scene, 1).unwrap();
    for (index, component) in scene.components.iter().enumerate() {
        let quantities: Vec<&str> = runtime
            .quantities(index)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        for (port, kind) in component.kind.ports() {
            let sink = kind == lab_model::scene::PortKind::DigitalSink;
            let commanded = component.kind.command_quantity(port);
            assert_eq!(commanded.is_some(), sink, "{}.{port}", component.id);
            if let Some(q) = commanded {
                assert!(
                    quantities.contains(&q),
                    "{}: {q} is not a runtime quantity",
                    component.id
                );
                assert!(runtime.is_commanded_quantity(index, q));
            }
        }
        // Quantities that are not the command of a sink never count as commands.
        for q in &quantities {
            let declared = component.kind.commanded_quantities().contains(q);
            assert_eq!(
                runtime.is_commanded_quantity(index, q),
                declared,
                "{}.{q}",
                component.id
            );
        }
    }
    let drive = 10;
    assert!(!runtime.is_commanded_quantity(drive, "running"));
    assert!(!runtime.is_commanded_quantity(drive, "remote"));
    assert!(runtime.is_commanded_quantity(drive, "command"));
}

/// The scene's drive with every optional terminal of the generic VFD model.
fn full_drive_scene() -> SceneRuntime {
    let mut scene = scene();
    for c in &mut scene.components {
        if let ComponentKind::VariableFrequencyDrive {
            enable_terminal,
            fault_terminals,
            speed_permille,
            preset_permille,
            keypad_permille,
            ..
        } = &mut c.kind
        {
            *enable_terminal = true;
            *fault_terminals = true;
            *speed_permille = 500;
            *preset_permille = Some(1000);
            *keypad_permille = 300;
        }
    }
    validate_scene(&scene).unwrap();
    SceneRuntime::new(&scene, 1).unwrap()
}

#[test]
fn the_generic_drive_needs_enable_follows_its_reference_and_latches_trips() {
    let mut s = full_drive_scene();
    let drive = s.index_of_str("drive").unwrap();
    let relay = |s: &SceneRuntime, port: &str| s.digital_output(drive, port).unwrap().conducting;
    // Without run enable the drive is not ready and ignores the run command.
    s.set_coil(drive, "run", true).unwrap();
    s.advance_to(at(500)).unwrap();
    assert!(!relay(&s, "ready") && !relay(&s, "running"));
    s.set_coil(drive, "enable", true).unwrap();
    assert!(relay(&s, "ready"));
    s.advance_to(at(2500)).unwrap();
    // Ramp 0..100 % in 2 s, limited to the 50 % terminal reference.
    assert_eq!(q(&s, "drive", "speedPermille"), Quantity::Integer(500));
    // The preset selects 100 %; the ramp continues from 50 %.
    s.set_coil(drive, "preset", true).unwrap();
    s.advance_to(at(3000)).unwrap();
    assert_eq!(q(&s, "drive", "speedPermille"), Quantity::Integer(750));
    s.advance_to(at(4000)).unwrap();
    assert_eq!(q(&s, "drive", "referencePermille"), Quantity::Integer(1000));
    assert_eq!(q(&s, "drive", "speedPermille"), Quantity::Integer(1000));
    // Keypad reference overrides the terminals.
    s.apply_fault(drive, ComponentFault::DriveKeypadReference)
        .unwrap();
    assert_eq!(q(&s, "drive", "speedPermille"), Quantity::Integer(300));
    s.clear_fault(drive, &ComponentFault::DriveKeypadReference)
        .unwrap();
    // A trip stops the drive and closes the fault relay; reset is refused while the cause
    // persists and acknowledges it afterwards.
    s.apply_fault(drive, ComponentFault::DriveTrip).unwrap();
    assert!(relay(&s, "fault") && !relay(&s, "ready") && !relay(&s, "running"));
    s.set_coil(drive, "reset", true).unwrap();
    s.set_coil(drive, "reset", false).unwrap();
    assert!(relay(&s, "fault"), "the cause is still there");
    s.clear_fault(drive, &ComponentFault::DriveTrip).unwrap();
    assert!(
        relay(&s, "fault"),
        "a trip stays latched until acknowledged"
    );
    s.set_coil(drive, "reset", true).unwrap();
    assert!(!relay(&s, "fault") && relay(&s, "ready") && relay(&s, "running"));
    // Losing the enable or the mains supply stops the drive.
    s.set_coil(drive, "enable", false).unwrap();
    assert!(!relay(&s, "running"));
    s.set_coil(drive, "enable", true).unwrap();
    s.apply_fault(drive, ComponentFault::DriveNoSupply).unwrap();
    assert!(!relay(&s, "ready") && !relay(&s, "running"));
    s.clear_fault(drive, &ComponentFault::DriveNoSupply)
        .unwrap();
    assert!(relay(&s, "running"));
}

#[test]
fn a_held_reset_acknowledges_nothing_later_and_undeclared_terminals_are_refused() {
    let mut s = full_drive_scene();
    let drive = s.index_of_str("drive").unwrap();
    let relay = |s: &SceneRuntime, port: &str| s.digital_output(drive, port).unwrap().conducting;
    s.set_coil(drive, "enable", true).unwrap();
    s.set_coil(drive, "reset", true).unwrap();
    s.apply_fault(drive, ComponentFault::DriveTrip).unwrap();
    s.clear_fault(drive, &ComponentFault::DriveTrip).unwrap();
    assert!(
        relay(&s, "fault"),
        "a reset held before the trip is no acknowledgement"
    );
    s.set_coil(drive, "reset", false).unwrap();
    s.set_coil(drive, "reset", true).unwrap();
    assert!(!relay(&s, "fault"), "a new rising edge acknowledges");
    // The plain drive of the base scene declares no enable terminal.
    let mut plain = SceneRuntime::new(&scene(), 1).unwrap();
    let d = plain.index_of_str("drive").unwrap();
    assert!(plain.set_coil(d, "enable", true).is_err());
    assert!(
        plain.set_coil(d, "ready", true).is_err(),
        "a relay output is no terminal"
    );
}

#[test]
fn drive_invariants_hold_for_every_sequence_of_terminals_and_faults() {
    let mut s = full_drive_scene();
    let drive = s.index_of_str("drive").unwrap();
    let faults = [
        ComponentFault::DriveLocalControl,
        ComponentFault::DriveTrip,
        ComponentFault::DriveKeypadReference,
        ComponentFault::DriveNoSupply,
    ];
    let mut active = [false; 4];
    let mut seed: u64 = 7;
    let mut now = 0;
    for _ in 0..3000 {
        // Deterministic pseudo-random sequence of terminal changes, faults and time.
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let pick = (seed >> 33) % 9;
        let on = (seed >> 20) & 1 == 1;
        match pick {
            0..=3 => {
                let port = ["run", "enable", "reset", "preset"][pick as usize];
                s.set_coil(drive, port, on).unwrap();
            }
            4..=7 => {
                let i = (pick - 4) as usize;
                if active[i] {
                    s.clear_fault(drive, &faults[i]).unwrap();
                } else {
                    s.apply_fault(drive, faults[i].clone()).unwrap();
                }
                active[i] = !active[i];
            }
            _ => {
                now += 100;
                s.advance_to(at(now)).unwrap();
            }
        }
        let b = |name: &str| q(&s, "drive", name) == Quantity::Bool(true);
        let Quantity::Integer(speed) = q(&s, "drive", "speedPermille") else {
            panic!()
        };
        let Quantity::Integer(reference) = q(&s, "drive", "referencePermille") else {
            panic!()
        };
        let relay = |port: &str| s.digital_output(drive, port).unwrap().conducting;
        if b("running") {
            assert!(
                b("ready") && b("remote") && b("command"),
                "running needs ready, remote, run"
            );
        }
        if b("ready") {
            assert!(b("supplied") && !b("tripped") && b("enableCommand"));
        }
        if !b("supplied") {
            assert!(!relay("ready") && !relay("running") && !relay("remote") && !relay("fault"));
        }
        assert!((0..=reference).contains(&speed) && reference <= 1000);
        if !b("running") {
            assert_eq!(speed, 0);
        }
    }
}

#[test]
fn drive_speed_values_are_validated_as_per_mille() {
    let mut scene = scene();
    for c in &mut scene.components {
        if let ComponentKind::VariableFrequencyDrive { speed_permille, .. } = &mut c.kind {
            *speed_permille = 1200;
        }
    }
    let error = validate_scene(&scene).unwrap_err().to_string();
    assert!(error.contains("per mille"), "{error}");
}
