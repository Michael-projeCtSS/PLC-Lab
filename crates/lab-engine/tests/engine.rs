//! Engine integration tests over the product scenes in the knowledge pack.
use lab_engine::{
    AlarmPriority, AlarmState, AlarmTransition, CycleOutcome, EngineError, ForceTarget,
    LabEventKind, LabSession, OperatorAction, PlcMode, SessionConfig, SignalId, SignalKind,
    SignalValue, StartKind,
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

#[test]
fn mode_changes_and_downloads_are_recorded_in_the_cpu_diagnostic_buffer() {
    use hardware_model::{CpuState, DiagnosticEvent, DiagnosticKind, DiagnosticSource};
    let mut s = session(TANK);
    s.download(TANK_FILL, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    s.step_cycle().unwrap();
    s.set_plc_switch(PlcModeRequest::Stop).unwrap();
    let events: Vec<DiagnosticKind> = s
        .station()
        .diagnostics()
        .entries()
        .filter(|e| e.event == DiagnosticEvent::Instant)
        .inspect(|e| assert_eq!(e.source, DiagnosticSource::Cpu))
        .map(|e| e.kind)
        .collect();
    let run = DiagnosticKind::ModeChange {
        from: CpuState::Stop,
        to: CpuState::Run,
    };
    let stop = DiagnosticKind::ModeChange {
        from: CpuState::Run,
        to: CpuState::Stop,
    };
    // The first start of a downloaded program is cold; its first scan follows (ADR-0038).
    let cold = hardware_model::StartupKind::Cold;
    let started = DiagnosticKind::Startup {
        kind: cold,
        retained: 0,
    };
    let first = DiagnosticKind::FirstScan { kind: cold };
    assert_eq!(
        events,
        [DiagnosticKind::ProgramDownloaded, started, run, first, stop],
        "single events, oldest first"
    );
    // Single events are not active conditions.
    let active = s.station().active_diagnostics().count();
    assert_eq!(active, 0);
}

const TANK_OVERFILL: &str = "PROGRAM Main
VAR_INPUT StartPB : BOOL; StopPB : BOOL; LevelLow : BOOL; LevelHigh : BOOL; InletOpen : BOOL; LevelRaw : INT; END_VAR
VAR_OUTPUT InletValve : BOOL; DrainPump : BOOL; END_VAR
InletValve := TRUE;
DrainPump := FALSE;
END_PROGRAM";
const TANK_DRAIN: &str = "PROGRAM Main
VAR_INPUT StartPB : BOOL; StopPB : BOOL; LevelLow : BOOL; LevelHigh : BOOL; InletOpen : BOOL; LevelRaw : INT; END_VAR
VAR_OUTPUT InletValve : BOOL; DrainPump : BOOL; END_VAR
InletValve := FALSE;
DrainPump := TRUE;
END_PROGRAM";

fn alarm_history(s: &LabSession, id: &str) -> Vec<(AlarmTransition, AlarmState)> {
    let entries = s.alarms().history().filter(|e| e.definition.id == id);
    entries.map(|e| (e.transition, e.to)).collect()
}

#[test]
fn a_process_alarm_follows_its_condition_and_acknowledgment_never_changes_the_process() {
    let mut s = session(TANK);
    let high = "tank.high-level";
    let definition = &s.alarms().get(high).unwrap().definition;
    assert_eq!(
        (definition.priority, definition.code.as_str()),
        (AlarmPriority::Alarm, "HIGH_LEVEL")
    );
    assert!(s.alarms().get("tank.overflow").is_some());
    assert_eq!(s.alarms().get(high).unwrap().state, AlarmState::Normal);
    s.download(TANK_OVERFILL, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    // 15 L to the high-high level of 142.5 L at 10 L/s takes 12.75 s, plus 0.75 s because
    // the inlet opens over its 1.5 s travel.
    for _ in 0..200 {
        s.step_cycle().unwrap();
    }
    let status = s.alarms().get(high).unwrap();
    assert_eq!(status.state, AlarmState::ActiveUnacknowledged);
    assert_eq!(status.activated.unwrap().as_micros(), 13_500_000);
    // A condition that holds for many cycles is recorded once.
    assert_eq!(alarm_history(&s, high).len(), 1);

    // The operator acknowledges: the alarm stays active and the process is unchanged.
    let level = |s: &LabSession| {
        let tank = s.scene().index_of_str("tank").unwrap();
        s.scene().quantity(tank, "levelMl")
    };
    let before = level(&s);
    s.acknowledge_alarm(high).unwrap();
    assert_eq!(level(&s), before);
    s.step_cycle().unwrap();
    let status = s.alarms().get(high).unwrap();
    assert_eq!(
        (status.state, status.condition),
        (AlarmState::ActiveAcknowledged, true)
    );
    assert!(matches!(
        s.acknowledge_alarm(high),
        Err(EngineError::InvalidAction(_))
    ));
    assert!(matches!(
        s.acknowledge_alarm("tank.no-such-alarm"),
        Err(EngineError::InvalidAction(_))
    ));
    // Process alarms report the plant, not the PLC: STOP and a memory reset leave them.
    s.set_plc_switch(PlcModeRequest::Stop).unwrap();
    s.step_cycle().unwrap();
    s.reset_plc().unwrap();
    assert_eq!(
        s.alarms().get(high).unwrap().state,
        AlarmState::ActiveAcknowledged
    );
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    // The overflow alarm is still waiting for its acknowledgment.
    assert_eq!(s.acknowledge_all_alarms(), 1);

    // Only correcting the process clears the condition: the alarms return to normal.
    s.download(TANK_DRAIN, 2, None).unwrap();
    for _ in 0..100 {
        s.step_cycle().unwrap();
    }
    assert_eq!(s.alarms().get(high).unwrap().state, AlarmState::Normal);
    assert_eq!(
        alarm_history(&s, high),
        [
            (AlarmTransition::Activated, AlarmState::ActiveUnacknowledged),
            (
                AlarmTransition::Acknowledged,
                AlarmState::ActiveAcknowledged
            ),
            (AlarmTransition::ReturnedToNormal, AlarmState::Normal),
        ]
    );
    let logged = s
        .events()
        .iter()
        .filter(|(_, e)| matches!(&e.kind, LabEventKind::Alarm { id, .. } if id == high));
    assert_eq!(logged.count(), 3, "every transition is in the event log");

    // A world reset restores the initial plant: every alarm normal, a new history.
    s.reset_world().unwrap();
    assert!(
        s.alarms()
            .alarms()
            .iter()
            .all(|a| a.state == AlarmState::Normal)
    );
    assert_eq!(s.alarms().history().len(), 0);
}

#[test]
fn the_same_run_records_the_same_alarm_history() {
    let run = || {
        let mut s = session(TANK);
        s.download(TANK_OVERFILL, 1, None).unwrap();
        s.set_plc_switch(PlcModeRequest::Run).unwrap();
        for _ in 0..160 {
            s.step_cycle().unwrap();
        }
        s.acknowledge_all_alarms();
        s.download(TANK_DRAIN, 2, None).unwrap();
        for _ in 0..80 {
            s.step_cycle().unwrap();
        }
        s.alarms()
            .history()
            .map(|e| (e.seq, e.time, e.definition.id.clone(), e.transition))
            .collect::<Vec<_>>()
    };
    let first = run();
    assert!(first.len() >= 6, "{first:?}");
    assert_eq!(first, run());
}

const RESTART: &str = "PROGRAM Main
VAR_INPUT StartPB : BOOL; StopPB : BOOL; LevelLow : BOOL; LevelHigh : BOOL; InletOpen : BOOL; LevelRaw : INT; END_VAR
VAR_OUTPUT InletValve : BOOL; DrainPump : BOOL; END_VAR
VAR RETAIN Presses : INT := 0; END_VAR
VAR Since : INT := 0; Starts : INT; Edge : R_TRIG; END_VAR
Edge(CLK := StartPB);
IF Edge.Q THEN Presses := Presses + 1; Since := Since + 1; END_IF;
IF FIRST_SCAN THEN Starts := Starts + 1; END_IF;
InletValve := Presses > 1;
DrainPump := FALSE;
END_PROGRAM";

fn press(s: &mut LabSession) {
    s.operate(OperatorAction::Press {
        component: "start".into(),
    })
    .unwrap();
    s.step_cycle().unwrap();
    s.operate(OperatorAction::Release {
        component: "start".into(),
    })
    .unwrap();
    s.step_cycle().unwrap();
}
fn committed(s: &LabSession, name: &str) -> SignalValue {
    let signal = signal(s, &format!("plc.{name}"));
    let (_, rows) = s.variables(Some(name), 0, usize::MAX);
    let row = rows.into_iter().find(|r| r.signal == signal).unwrap();
    row.committed.unwrap()
}
fn startups(s: &LabSession) -> Vec<hardware_model::DiagnosticKind> {
    use hardware_model::DiagnosticKind;
    let entries = s.station().diagnostics().entries();
    entries
        .map(|e| e.kind)
        .filter(|k| {
            matches!(
                k,
                DiagnosticKind::Startup { .. }
                    | DiagnosticKind::FirstScan { .. }
                    | DiagnosticKind::MemoryReset
            )
        })
        .collect()
}

#[test]
fn warm_and_cold_restarts_initialize_memory_once_and_report_the_first_scan() {
    use hardware_model::{DiagnosticKind, StartupKind};
    let mut s = session(TANK);
    s.download(RESTART, 1, None).unwrap();
    // A restart needs a running CPU.
    assert!(matches!(
        s.restart(StartKind::Warm),
        Err(EngineError::InvalidAction(_))
    ));
    // In STOP the first scan is not consumed.
    s.step_cycle().unwrap();
    assert!(s.first_scan_pending());
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    assert_eq!(
        s.last_start(),
        Some(StartKind::Cold),
        "a download starts cold"
    );
    s.step_cycle().unwrap();
    assert!(!s.first_scan_pending());
    press(&mut s);
    press(&mut s);
    assert_eq!(committed(&s, "Presses"), SignalValue::Int(2));
    assert_eq!(committed(&s, "Since"), SignalValue::Int(2));
    assert_eq!(committed(&s, "Starts"), SignalValue::Int(1));
    assert_eq!(s.retained_variables(), 1);

    // A forced output survives a restart and stays listed.
    let valve = signal(&s, "plc.InletValve");
    s.force(valve, Some(SignalValue::Bool(false))).unwrap();
    // Warm restart: the retained count stays, the rest starts again.
    assert_eq!(s.restart(StartKind::Warm).unwrap(), PlcMode::Run);
    assert_eq!(s.last_start(), Some(StartKind::Warm));
    assert!(s.first_scan_pending());
    assert_eq!(committed(&s, "Presses"), SignalValue::Int(2));
    assert_eq!(committed(&s, "Since"), SignalValue::Int(0));
    assert_eq!(s.forces().len(), 1, "forces survive a restart");
    s.step_cycle().unwrap();
    assert_eq!(
        committed(&s, "Starts"),
        SignalValue::Int(1),
        "Starts is not retained"
    );
    // A STOP to RUN transition is a warm start too.
    s.set_plc_switch(PlcModeRequest::Stop).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    assert_eq!(s.last_start(), Some(StartKind::Warm));
    s.step_cycle().unwrap();
    assert_eq!(committed(&s, "Presses"), SignalValue::Int(2));

    // Cold restart: everything is initialized, retained variables included.
    s.restart(StartKind::Cold).unwrap();
    assert_eq!(committed(&s, "Presses"), SignalValue::Int(0));
    s.step_cycle().unwrap();
    assert_eq!(s.forces().len(), 1);

    // A memory reset clears forces and prepares a cold start.
    press(&mut s);
    s.reset_plc().unwrap();
    assert!(s.forces().is_empty());
    assert_eq!(committed(&s, "Presses"), SignalValue::Int(0));
    // With the mode switch in RUN the CPU starts again at once, cold.
    assert_eq!(
        (s.plc_mode(), s.last_start()),
        (PlcMode::Run, Some(StartKind::Cold))
    );
    s.step_cycle().unwrap();

    let (warm, cold) = (StartupKind::Warm, StartupKind::Cold);
    let start = |kind, retained| DiagnosticKind::Startup { kind, retained };
    let first = |kind| DiagnosticKind::FirstScan { kind };
    assert_eq!(
        startups(&s),
        [
            start(cold, 1),
            first(cold),
            start(warm, 1),
            first(warm),
            start(warm, 1),
            first(warm),
            start(cold, 1),
            first(cold),
            DiagnosticKind::MemoryReset,
            start(cold, 1),
            first(cold),
        ],
        "one start and one first scan each, never per scan"
    );
}

#[test]
fn a_restart_does_not_touch_the_process_and_a_faulted_cpu_needs_a_memory_reset() {
    let mut s = session(TANK);
    s.download(RESTART, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    for _ in 0..3 {
        press(&mut s);
    }
    for _ in 0..20 {
        s.step_cycle().unwrap();
    }
    let level = |s: &LabSession| {
        let tank = s.scene().index_of_str("tank").unwrap();
        s.scene().quantity(tank, "levelMl")
    };
    let before = level(&s);
    s.restart(StartKind::Cold).unwrap();
    assert_eq!(level(&s), before, "a restart changes no physical state");
    s.download(
        "PROGRAM Main VAR_OUTPUT InletValve : BOOL; END_VAR VAR X : INT; END_VAR
         X := 32767; X := X + 1; InletValve := FALSE; END_PROGRAM",
        2,
        None,
    )
    .unwrap();
    s.step_cycle().unwrap();
    assert_eq!(s.plc_mode(), PlcMode::Fault);
    assert!(matches!(
        s.restart(StartKind::Warm),
        Err(EngineError::PlcFaulted)
    ));
}

#[test]
fn the_same_restarts_give_the_same_trace() {
    let run = || {
        let mut s = session(TANK);
        s.download(RESTART, 1, None).unwrap();
        s.set_plc_switch(PlcModeRequest::Run).unwrap();
        press(&mut s);
        s.restart(StartKind::Warm).unwrap();
        press(&mut s);
        s.restart(StartKind::Cold).unwrap();
        press(&mut s);
        let values: Vec<_> = ["Presses", "Since", "Starts"]
            .iter()
            .map(|n| committed(&s, n))
            .collect();
        (values, startups(&s), s.cycle())
    };
    assert_eq!(run(), run());
}
