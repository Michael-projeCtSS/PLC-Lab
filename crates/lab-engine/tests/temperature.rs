//! Closed-loop temperature control in the real engine (ADR-0046, ADR-0047): the student's
//! ST program with the PID block, the analog input and output modules and the heated
//! vessel, cycle by cycle.
use lab_engine::{LabSession, OperatorAction, SessionConfig};
use lab_model::{
    exercise::PlcModeRequest, parse_json, profile::PlatformProfile, scene::SceneDefinition,
};
use process_simulation::Quantity;

const SCENE: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/heater-loop.json");

fn program(kp: &str, ti: &str) -> String {
    format!(
        "PROGRAM Main
VAR_INPUT AutoMode : BOOL; TempRaw : INT; END_VAR
VAR_OUTPUT HeaterRaw : INT; HeaterLamp : BOOL; AlarmLamp : BOOL; END_VAR
VAR Loop : PID; Setpoint : REAL := 60.0; Temperature : REAL; Heater : REAL; END_VAR
Temperature := INT_TO_REAL(TempRaw) * 0.015;
Loop(SP := Setpoint, PV := Temperature, KP := {kp}, TI := {ti},
     MANUAL := NOT AutoMode, MV_MAN := 25.0, MV => Heater);
HeaterRaw := REAL_TO_INT(Heater * 100.0);
HeaterLamp := Heater > 0.5;
AlarmLamp := Temperature > 90.0;
END_PROGRAM"
    )
}

fn plant(source: &str) -> LabSession {
    let scene: SceneDefinition = parse_json(SCENE).unwrap();
    lab_model::validate_scene(&scene).unwrap();
    let station = scene.reference_station.clone();
    let mut s = LabSession::new(
        scene,
        station,
        PlatformProfile::generic(),
        SessionConfig::default(),
    )
    .unwrap();
    s.download(source, 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    s
}

fn quantity(s: &LabSession, name: &str) -> i64 {
    let oven = s.scene().index_of_str("oven").unwrap();
    match s.scene().quantity(oven, name).unwrap() {
        Quantity::Integer(i) => i,
        Quantity::Bool(b) => i64::from(b),
    }
}
fn auto(s: &mut LabSession, on: bool) {
    s.operate(OperatorAction::SetSelector {
        component: "sel".into(),
        position: u8::from(on),
    })
    .unwrap();
}
/// Run `seconds` and return the temperature (m°C) and heater power (‰) each second.
fn run(s: &mut LabSession, seconds: u64) -> Vec<(i64, i64)> {
    let per_second = 1_000_000 / s.config().cycle_us;
    (0..seconds)
        .map(|_| {
            for _ in 0..per_second {
                s.step_cycle().unwrap();
            }
            (
                quantity(s, "temperatureMilliC"),
                quantity(s, "powerPermille"),
            )
        })
        .collect()
}

fn door(s: &mut LabSession, open: bool) {
    s.operate(OperatorAction::SetDoor {
        component: "oven".into(),
        open,
    })
    .unwrap();
}

#[test]
fn a_pi_loop_heats_the_vessel_to_its_setpoint_and_rejects_the_open_door() {
    let mut s = plant(&program("3.0", "T#20s"));
    auto(&mut s, true);
    let start = run(&mut s, 300);
    let peak = start.iter().map(|x| x.0).max().unwrap();
    let reached = start.iter().position(|x| x.0 >= 59_000).unwrap();
    assert!(reached < 90, "59 °C after {reached} s");
    assert!(peak < 61_000, "overshoot to {peak} m°C");
    // At 60 °C the vessel needs 50 % heater power.
    assert!((start[299].0 - 60_000).abs() <= 100, "{:?}", start[299]);
    assert!((start[299].1 - 500).abs() <= 5, "{:?}", start[299]);
    // The open door is a disturbance: the temperature dips and the integral brings it
    // back with 75 % power.
    door(&mut s, true);
    assert_eq!(quantity(&s, "doorOpen"), 1);
    let open = run(&mut s, 200);
    let dip = open.iter().map(|x| x.0).min().unwrap();
    assert!((50_000..58_000).contains(&dip), "dip to {dip}");
    assert!((open[199].0 - 60_000).abs() <= 100, "{:?}", open[199]);
    assert!((open[199].1 - 750).abs() <= 10, "{:?}", open[199]);
}

#[test]
fn proportional_control_alone_leaves_the_predicted_offset() {
    let mut s = plant(&program("4.0", "T#0s"));
    auto(&mut s, true);
    let trace = run(&mut s, 300);
    // T = 20 + 0.8 * 4 * (60 - T) → 50.5 °C.
    assert!((trace[299].0 - 50_476).abs() <= 100, "{:?}", trace[299]);
}

#[test]
fn manual_mode_holds_its_output_and_automatic_takes_over_without_a_bump() {
    let mut s = plant(&program("3.0", "T#20s"));
    let manual = run(&mut s, 120);
    // MAN: 25 % power holds 20 + 0.8 * 25 = 40 °C.
    assert_eq!(manual[119].1, 250);
    assert!((manual[119].0 - 40_000).abs() <= 200, "{:?}", manual[119]);
    auto(&mut s, true);
    let after = run(&mut s, 3);
    // The output continues from 25 % and rises with the integral; no jump.
    assert!(after[0].1 < 320, "{after:?}");
    assert!(after.windows(2).all(|w| w[1].1 - w[0].1 < 60), "{after:?}");
}

#[test]
fn the_controller_is_visible_online_under_its_instance_path() {
    let mut s = plant(&program("3.0", "T#20s"));
    auto(&mut s, true);
    run(&mut s, 30);
    let (_, rows) = s.variables(Some("Loop."), 0, 100);
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    for part in [
        "Loop.MV", "Loop.E", "Loop.P", "Loop.I", "Loop.D", "Loop.SAT",
    ] {
        assert!(names.contains(&part), "{part} missing in {names:?}");
    }
}

#[test]
fn the_closed_loop_is_deterministic() {
    let trace = || {
        let mut s = plant(&program("2.0", "T#15s"));
        auto(&mut s, true);
        let mut t = run(&mut s, 120);
        door(&mut s, true);
        t.extend(run(&mut s, 60));
        t
    };
    assert_eq!(trace(), trace());
}
