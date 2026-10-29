//! Engine integration tests over the product scenes in the knowledge pack.
use lab_engine::{
    CycleOutcome, EngineError, ForceTarget, LabEventKind, LabSession, OperatorAction, PlcMode,
    SessionConfig, SignalId, SignalKind, SignalValue,
};
use lab_model::{
    exercise::PlcModeRequest,
    fault::{FaultInjection, FaultSpec},
    hardware::{ChannelMode, SlotRef},
    network::Ipv4,
    parse_json,
    profile::PlatformProfile,
    scene::SceneDefinition,
};

const TANK: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/tank-filling.json");
const MOTOR: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/motor-start-stop.json");
const MODBUS: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/tank-modbus.json");

fn scene(text: &str) -> SceneDefinition {
    parse_json(text).unwrap()
}
fn session(text: &str) -> LabSession {
    let s = scene(text);
    let station = s.reference_station.clone();
    LabSession::new(
        s,
        station,
        PlatformProfile::generic(),
        SessionConfig::default(),
    )
    .unwrap()
}
fn signal(s: &LabSession, path: &str) -> SignalId {
    s.signals()
        .by_path(path)
        .unwrap_or_else(|| panic!("missing signal {path}"))
}
fn value(layer: &[(SignalId, SignalValue)], id: SignalId) -> SignalValue {
    layer
        .iter()
        .find(|(s, _)| *s == id)
        .map(|(_, v)| *v)
        .unwrap()
}

const TANK_FILL: &str = "PROGRAM Main
VAR_INPUT StartPB : BOOL; StopPB : BOOL; LevelLow : BOOL; LevelHigh : BOOL; InletOpen : BOOL; LevelRaw : INT; END_VAR
VAR_OUTPUT InletValve : BOOL; DrainPump : BOOL; END_VAR
InletValve := NOT LevelHigh;
DrainPump := FALSE;
END_PROGRAM";

#[test]
fn causal_invariant_holds_inside_every_single_observation() {
    let mut s = session(TANK);
    s.download(TANK_FILL, 1, None).unwrap();
    assert_eq!(s.set_plc_switch(PlcModeRequest::Run).unwrap(), PlcMode::Run);
    let high = signal(&s, "plc.LevelHigh");
    let valve = signal(&s, "plc.InletValve");
    let mut transitions = 0;
    let mut previous_high = None;
    for _ in 0..200 {
        let o = s.step_cycle().unwrap();
        assert_eq!(o.header.outcome, CycleOutcome::Scanned);
        let SignalValue::Bool(saw) = value(&o.scan_inputs, high) else {
            panic!()
        };
        // The audit's blocker: the decision must be provable from this row alone.
        assert_eq!(
            value(&o.committed, valve),
            SignalValue::Bool(!saw),
            "cycle {}",
            o.header.cycle
        );
        if previous_high.is_some_and(|p| p != saw) {
            transitions += 1;
        }
        // The next row's scan inputs equal this row's staged inputs (no operator action).
        previous_high = Some(saw);
        if let SignalValue::Bool(next) = value(&o.staged_next, high)
            && next != saw
        {
            let ce = s.latest_cause_effect().unwrap();
            assert!(
                ce.next.iter().any(|e| e.signal == high && e.changed),
                "cause/effect flags the next-scan change"
            );
        }
    }
    assert!(
        transitions >= 1,
        "the high switch must be reached and acted upon"
    );
    // The tank never overflows: the valve closes one scan after the switch is wet.
    let overflow = s.events().iter().any(|(_, e)| {
        matches!(
            &e.kind,
            LabEventKind::Process {
                kind: process_simulation::ProcessEventKind::OverflowStarted,
                ..
            }
        )
    });
    assert!(!overflow);
}

#[test]
fn identical_sessions_produce_identical_observations() {
    let run = || {
        let mut s = session(TANK);
        s.download(TANK_FILL, 1, None).unwrap();
        s.set_plc_switch(PlcModeRequest::Run).unwrap();
        (0..150)
            .map(|_| s.step_cycle().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn plc_stop_de_energizes_outputs_while_the_world_keeps_running() {
    let mut s = session(TANK);
    s.download(
        "PROGRAM Main VAR_INPUT LevelLow : BOOL; END_VAR VAR_OUTPUT DrainPump : BOOL; InletValve : BOOL; END_VAR
         DrainPump := TRUE; InletValve := FALSE; END_PROGRAM",
        1,
        None,
    )
    .unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    let level = signal(&s, "process.tank.levelMl");
    let first = s.step_cycle().unwrap();
    let draining = s.step_cycle().unwrap();
    assert!(draining.process_after.iter().any(|(id, v)| *id == level && v.as_f64() < value(&first.process_after, level).as_f64()));
    s.set_plc_switch(PlcModeRequest::Stop).unwrap();
    let a = s.step_cycle().unwrap();
    let b = s.step_cycle().unwrap();
    assert_eq!(a.header.outcome, CycleOutcome::NotScanned(PlcMode::Stop));
    assert!(
        b.header.start_us > a.header.start_us,
        "world time advances in STOP"
    );
    assert_eq!(
        value(&a.process_after, level),
        value(&b.process_after, level),
        "pump stopped with the CPU"
    );
    // PLC task time does not advance in STOP; world time does.
    let task = s.plc_task_time_us();
    s.step_cycle().unwrap();
    assert_eq!(s.plc_task_time_us(), task);
}

#[test]
fn faulted_scan_commits_nothing_freezes_that_step_and_requires_reset() {
    let mut s = session(TANK);
    s.download(
        "PROGRAM Main VAR_INPUT LevelRaw : INT; END_VAR VAR_OUTPUT InletValve : BOOL; END_VAR VAR Q : INT; C : CTU; END_VAR
         C(CU := TRUE, PV := 5);
         InletValve := TRUE;
         Q := 100 / (LevelRaw - LevelRaw);
         END_PROGRAM",
        1,
        None,
    )
    .unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    let start = s.now();
    let o = s.step_cycle().unwrap();
    assert_eq!(o.header.outcome, CycleOutcome::Faulted);
    assert_eq!(o.fault.as_ref().unwrap().code(), "PLC-RUNTIME-003");
    assert_eq!(
        s.now(),
        start,
        "the faulting step does not advance the world"
    );
    assert_eq!(s.plc_mode(), PlcMode::Fault);
    assert_eq!(
        value(&o.committed, signal(&s, "plc.InletValve")),
        SignalValue::Bool(false),
        "nothing committed"
    );
    assert_eq!(
        value(&o.committed, signal(&s, "plc.C.CV")),
        SignalValue::Int(0),
        "block state rolled back"
    );
    let next = s.step_cycle().unwrap();
    assert_eq!(
        next.header.outcome,
        CycleOutcome::NotScanned(PlcMode::Fault)
    );
    assert!(
        s.now() > start,
        "the plant keeps evolving while the CPU is in FAULT"
    );
    assert!(matches!(
        s.set_plc_switch(PlcModeRequest::Run),
        Err(EngineError::PlcFaulted)
    ));
    let world = s.now();
    s.reset_plc().unwrap();
    assert_eq!(s.now(), world, "a PLC reset never rewinds world time");
}

#[test]
fn operator_actions_reach_the_next_scan_but_never_rewrite_history() {
    let mut s = session(MOTOR);
    s.download(
        "PROGRAM Main VAR_INPUT StartPB : BOOL; StopPB : BOOL; MotorFeedback : BOOL; OverloadOK : BOOL; END_VAR
         VAR_OUTPUT MotorContactor : BOOL; RunLamp : BOOL; END_VAR
         MotorContactor := (StartPB OR MotorContactor) AND StopPB AND OverloadOK;
         RunLamp := MotorFeedback;
         END_PROGRAM",
        1,
        None,
    )
    .unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    let start = signal(&s, "plc.StartPB");
    let contactor = signal(&s, "plc.MotorContactor");
    let lamp = signal(&s, "plc.RunLamp");
    let idle = s.step_cycle().unwrap();
    s.operate(OperatorAction::Press {
        component: "start".into(),
    })
    .unwrap();
    assert_eq!(
        value(&s.latest().unwrap().scan_inputs, start),
        SignalValue::Bool(false),
        "history is unchanged"
    );
    let pressed = s.step_cycle().unwrap();
    assert_eq!(value(&pressed.scan_inputs, start), SignalValue::Bool(true));
    assert_eq!(
        value(&pressed.committed, contactor),
        SignalValue::Bool(true)
    );
    s.operate(OperatorAction::Release {
        component: "start".into(),
    })
    .unwrap();
    let held = s.step_cycle().unwrap();
    assert_eq!(
        value(&held.committed, contactor),
        SignalValue::Bool(true),
        "latched"
    );
    // Feedback arrives after the contactor pick-up time and lights the lamp one scan later.
    let lamp_on = s.step_cycle().unwrap();
    assert_eq!(value(&lamp_on.committed, lamp), SignalValue::Bool(true));
    s.operate(OperatorAction::Press {
        component: "stop".into(),
    })
    .unwrap();
    let stopped = s.step_cycle().unwrap();
    assert_eq!(
        value(&stopped.committed, contactor),
        SignalValue::Bool(false)
    );
    assert_eq!(idle.header.cycle + 4, stopped.header.cycle);
    assert!(
        s.events()
            .iter()
            .any(|(_, e)| matches!(e.kind, LabEventKind::Operator { .. }))
    );
}

#[test]
fn hardware_fault_and_repair_are_diagnosable_through_the_station() {
    let mut s = session(MOTOR);
    s.download("PROGRAM Main VAR_INPUT StartPB : BOOL; END_VAR VAR_OUTPUT MotorContactor : BOOL; END_VAR MotorContactor := StartPB; END_PROGRAM", 1, None)
        .unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    let fault = FaultInjection {
        id: lab_model::Identifier::new("di-moved").unwrap(),
        fault: FaultSpec::ModuleInWrongSlot {
            from: SlotRef { rack: 0, slot: 1 },
            to: SlotRef { rack: 0, slot: 4 },
        },
    };
    s.inject_fault(&fault).unwrap();
    s.operate(OperatorAction::Press {
        component: "start".into(),
    })
    .unwrap();
    let o = s.step_cycle().unwrap();
    assert_eq!(
        value(&o.committed, signal(&s, "plc.MotorContactor")),
        SignalValue::Bool(false)
    );
    let (_, rows) = s.variables(Some("StartPB"), 0, 10);
    assert_eq!(rows[0].quality, Some(hardware_model::Quality::ModuleFault));
    assert!(
        s.station()
            .module_views()
            .iter()
            .any(|m| m.status == hardware_model::ModuleStatus::Unconfigured)
    );
    assert_eq!(
        s.repair(
            "Moved DI module back to slot 1",
            std::slice::from_ref(&fault.id)
        )
        .unwrap(),
        1
    );
    let fixed = s.step_cycle().unwrap();
    assert_eq!(
        value(&fixed.committed, signal(&s, "plc.MotorContactor")),
        SignalValue::Bool(true)
    );
}

#[test]
fn analog_range_configuration_changes_the_plc_view_of_the_same_physics() {
    let mut s = session(TANK);
    let program = "PROGRAM Main VAR_INPUT LevelRaw : INT; END_VAR VAR_OUTPUT InletValve : BOOL; END_VAR VAR Level : REAL; END_VAR
                   Level := INT_TO_REAL(LevelRaw) / 100.0; InletValve := FALSE; END_PROGRAM";
    s.download(program, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    let raw = signal(&s, "plc.LevelRaw");
    let o = s.step_cycle().unwrap();
    // Generic profile: 10 % level -> 5.6 mA -> 1000 per ten thousand.
    assert_eq!(value(&o.scan_inputs, raw), SignalValue::Int(1000));
    let mut wrong = s.station().configuration().clone();
    wrong.modules[2].channels[0].mode = ChannelMode::Current0To20mA;
    s.download(program, 2, Some(wrong)).unwrap();
    s.step_cycle().unwrap();
    let o = s.step_cycle().unwrap();
    assert_eq!(
        value(&o.scan_inputs, raw),
        SignalValue::Int(2800),
        "5.6 mA read as 0-20 mA"
    );
}

#[test]
fn modbus_scan_feeds_plc_inputs_and_a_wrong_subnet_is_observable() {
    let mut s = session(MODBUS);
    let program = "PROGRAM Main VAR_INPUT LevelPermille : INT; LevelPercent : REAL; LevelCommOK : BOOL; END_VAR
                   VAR_OUTPUT InletValve : BOOL; END_VAR InletValve := FALSE; END_PROGRAM";
    s.download(program, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    let mut last = None;
    for _ in 0..5 {
        last = Some(s.step_cycle().unwrap());
    }
    let o = last.take().unwrap();
    assert_eq!(
        value(&o.scan_inputs, signal(&s, "plc.LevelCommOK")),
        SignalValue::Bool(true)
    );
    assert_eq!(
        value(&o.scan_inputs, signal(&s, "plc.LevelPermille")),
        SignalValue::Int(100)
    );
    assert_eq!(
        value(&o.scan_inputs, signal(&s, "plc.LevelPercent")),
        SignalValue::Real(10.0)
    );
    let mut wrong = s.station().configuration().clone();
    wrong.network.interface.as_mut().unwrap().ip = Ipv4::parse("192.168.1.10").unwrap();
    s.download(program, 2, Some(wrong)).unwrap();
    for _ in 0..5 {
        last = Some(s.step_cycle().unwrap());
    }
    assert_eq!(
        value(&last.unwrap().scan_inputs, signal(&s, "plc.LevelCommOK")),
        SignalValue::Bool(false)
    );
    assert!(
        s.station()
            .active_diagnostics()
            .any(|(_, k)| *k == hardware_model::DiagnosticKind::NoRoute)
    );
    assert_eq!(
        s.network().ping(Ipv4::parse("192.168.0.20").unwrap()),
        network_core::PingResult::NoRoute
    );
}

#[test]
fn binding_errors_block_downloads_and_keep_the_running_program() {
    let mut s = session(MOTOR);
    s.download("PROGRAM Main VAR_INPUT StartPB : BOOL; END_VAR VAR_OUTPUT MotorContactor : BOOL; END_VAR MotorContactor := StartPB; END_PROGRAM", 1, None)
        .unwrap();
    let result = s.download(
        "PROGRAM Main VAR_INPUT StartPB : INT; END_VAR END_PROGRAM",
        2,
        None,
    );
    let Err(EngineError::Binding(issues)) = result else {
        panic!("binding error expected")
    };
    assert!(issues.iter().any(|i| i.blocking && i.tag == "StartPB"));
    assert_eq!(s.program().unwrap().1, 1);
    let Err(EngineError::Compile(_)) = s.download("PROGRAM Main x := ; END_PROGRAM", 3, None)
    else {
        panic!()
    };
}

#[test]
fn trace_stays_within_its_byte_budget_for_large_programs() {
    let mut decls = String::new();
    let mut body = String::new();
    for i in 0..1500 {
        decls.push_str(&format!("V{i} : BOOL; "));
        if i % 50 == 0 {
            body.push_str(&format!("V{i} := NOT V{i}; "));
        }
    }
    let program = format!(
        "PROGRAM Main VAR_INPUT LevelHigh : BOOL; END_VAR VAR_OUTPUT InletValve : BOOL; END_VAR VAR {decls} END_VAR {body} InletValve := NOT LevelHigh; END_PROGRAM"
    );
    let scene = scene(TANK);
    let station = scene.reference_station.clone();
    let config = SessionConfig {
        trace_bytes: 512 * 1024,
        ..SessionConfig::default()
    };
    let mut s = LabSession::new(scene, station, PlatformProfile::generic(), config).unwrap();
    s.download(&program, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    for _ in 0..3000 {
        s.step_cycle().unwrap();
    }
    let stats = s.trace_stats();
    assert!(
        stats.used_bytes <= 512 * 1024 + 300 * 1024,
        "used {} bytes",
        stats.used_bytes
    );
    assert!(stats.dropped_records > 0);
    assert_eq!(stats.dropped_records + stats.retained_records as u64, 3000);
    let locals = s
        .signals()
        .iter()
        .filter(|d| d.kind == SignalKind::PlcLocal)
        .count();
    assert_eq!(locals, 1500);
    // Trend queries over retained history are bounded.
    let level = signal(&s, "process.tank.levelMl");
    let series = s.trend(&[level], 0, u64::MAX, 200);
    assert!(series[0].points.len() <= 200);
    let latest = s.latest().unwrap().header.cycle;
    assert!(s.observation(latest).is_some());
    assert!(
        s.observation(0).is_none(),
        "evicted history is reported missing, not invented"
    );
}

#[test]
fn world_reset_restores_the_plant_and_keeps_program_and_signal_ids() {
    let mut s = session(TANK);
    s.download(TANK_FILL, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    for _ in 0..40 {
        s.step_cycle().unwrap();
    }
    let level = signal(&s, "process.tank.levelMl");
    let valve = signal(&s, "plc.InletValve");
    s.reset_world().unwrap();
    assert_eq!(s.now().as_micros(), 0);
    assert_eq!(s.signals().by_path("process.tank.levelMl"), Some(level));
    assert_eq!(s.signals().by_path("plc.InletValve"), Some(valve));
    assert_eq!(s.plc_mode(), PlcMode::Run);
    let o = s.step_cycle().unwrap();
    assert_eq!(
        value(&o.scan_inputs, signal(&s, "plc.LevelHigh")),
        SignalValue::Bool(false)
    );
}

/// Components whose identifiers look like quantity names: the applied layer must follow
/// the kinds' port metadata, not any name.
const MISLEADING: &str = r#"{
  "format": "plclab-scene", "schemaVersion": 1, "id": "test.misleading-names", "revision": 1,
  "title": "Names", "summary": "Identifiers that look like quantities",
  "components": [
    {"id": "command", "label": "Lamp called command", "kind": {"type": "pilotLamp", "color": "green"}},
    {"id": "lit", "label": "Starter called lit", "kind": {"type": "motorStarter", "pickupMs": 30, "runUpMs": 500, "ratedCurrentMa": 2000}},
    {"id": "running", "label": "Drive called running", "kind": {"type": "variableFrequencyDrive", "rampMs": 1000, "ratedCurrentMa": 3000}}
  ],
  "installation": {"cpu": "cpu.compact", "modules": [
      {"position": {"rack": 0, "slot": 2}, "module": "do8.24vdc"}
    ], "wiring": [
      {"channel": {"rack": 0, "slot": 2, "channel": 0}, "component": "command", "port": "coil"},
      {"channel": {"rack": 0, "slot": 2, "channel": 1}, "component": "lit", "port": "coil"},
      {"channel": {"rack": 0, "slot": 2, "channel": 2}, "component": "running", "port": "run"}
    ]},
  "referenceStation": {"name": "PLC_1", "cpu": "cpu.compact", "modules": [
      {"position": {"rack": 0, "slot": 2}, "module": "do8.24vdc", "startAddress": 0}
    ], "ioMap": [
      {"channel": {"rack": 0, "slot": 2, "channel": 0}, "tag": "Lamp"},
      {"channel": {"rack": 0, "slot": 2, "channel": 1}, "tag": "Coil"},
      {"channel": {"rack": 0, "slot": 2, "channel": 2}, "tag": "Run"}
    ]},
  "interface": [],
  "layout": []
}"#;

#[test]
fn applied_layer_follows_port_metadata_not_names() {
    let mut s = session(MISLEADING);
    s.download(
        "PROGRAM Main VAR_OUTPUT Lamp : BOOL; Coil : BOOL; Run : BOOL; END_VAR Lamp := TRUE; Coil := TRUE; Run := TRUE; END_PROGRAM",
        1,
        None,
    )
    .unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    let o = s.step_cycle().unwrap();
    let applied: Vec<String> = o
        .applied
        .iter()
        .filter_map(|(id, _)| s.signals().get(*id).map(|d| d.path.clone()))
        .filter(|p| p.starts_with("process."))
        .collect();
    // One commanded quantity per wired sink, chosen by component kind.
    assert_eq!(
        applied,
        [
            "process.command.lit",
            "process.lit.command",
            "process.running.command"
        ]
    );
    for (path, expected) in [
        ("process.command.lit", true),
        ("process.lit.command", true),
        ("process.running.command", true),
    ] {
        assert_eq!(
            value(&o.applied, signal(&s, path)),
            SignalValue::Bool(expected),
            "{path}"
        );
    }
}

#[test]
fn forces_are_typed_survive_stop_and_run_and_end_with_a_memory_reset() {
    let mut s = session(TANK);
    s.download(TANK_FILL, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    let raw = signal(&s, "plc.LevelRaw");
    let valve = signal(&s, "plc.InletValve");
    s.force(raw, Some(SignalValue::Int(1234))).unwrap();
    s.force(valve, Some(SignalValue::Bool(false))).unwrap();
    assert!(
        s.force(raw, Some(SignalValue::Bool(true))).is_err(),
        "a force keeps the tag's type"
    );
    s.step_cycle().unwrap();
    let scanned = |s: &LabSession| s.variables(Some("LevelRaw"), 0, 10).1[0].scan_value;
    assert_eq!(scanned(&s), Some(SignalValue::Int(1234)));
    let mut targets: Vec<_> = s
        .active_forces()
        .iter()
        .map(|f| (f.signal, f.target))
        .collect();
    targets.sort_by_key(|(signal, _)| *signal);
    let mut expected = vec![
        (raw, ForceTarget::InputImage),
        (valve, ForceTarget::OutputImage),
    ];
    expected.sort_by_key(|(signal, _)| *signal);
    assert_eq!(targets, expected);
    // STOP and RUN keep the forces.
    s.set_plc_switch(PlcModeRequest::Stop).unwrap();
    s.step_cycle().unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    s.step_cycle().unwrap();
    assert_eq!(s.active_forces().len(), 2);
    // A memory reset removes every force, and the event log says so.
    let removal = |kind: &LabEventKind| matches!(kind, LabEventKind::Force { value: None, .. });
    let removals = |s: &LabSession| {
        let events = s.events().iter();
        events.filter(|(_, e)| removal(&e.kind)).count()
    };
    s.reset_plc().unwrap();
    assert!(s.forces().is_empty());
    assert_eq!(removals(&s), 2);
    // Remove all forces in one request.
    s.force(raw, Some(SignalValue::Int(5))).unwrap();
    s.force(valve, Some(SignalValue::Bool(true))).unwrap();
    assert_eq!(s.remove_all_forces().unwrap(), 2);
    assert!(s.active_forces().is_empty());
    assert_eq!(removals(&s), 4);
    // A download without the forced symbol ends its force.
    s.force(valve, Some(SignalValue::Bool(true))).unwrap();
    s.download(
        "PROGRAM Main VAR_INPUT LevelRaw : INT; END_VAR VAR_OUTPUT DrainPump : BOOL; END_VAR
         DrainPump := FALSE; END_PROGRAM",
        2,
        None,
    )
    .unwrap();
    assert!(s.forces().is_empty());
    assert_eq!(removals(&s), 5);
}
