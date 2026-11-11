//! A Ladder Diagram program in the real engine (ADR-0050, ADR-0051): loaded like ST, linked
//! to the station's I/O, driving the real motor, and observed online from one recorded
//! cycle at a time.
use lab_engine::{LabSession, OperatorAction, SessionConfig, SignalValue};
use lab_model::{
    exercise::PlcModeRequest, parse_json, profile::PlatformProfile, scene::SceneDefinition,
};

const SCENE: &str =
    include_str!("../../../knowledge/PLC_Lab_Knowledge_Pack_v1/scenes/motor-start-stop.json");

/// Start in parallel with the held contactor, Stop (NC wiring: TRUE while not pressed) and
/// the overload contact in series, then a TON that delays the run lamp by 300 ms.
fn ladder() -> &'static str {
    r##"{
  "format": "plclab-lad",
  "version": 2,
  "name": "Main",
  "variables": [
    {
      "name": "StartPB",
      "type": "BOOL",
      "direction": "input"
    },
    {
      "name": "StopPB",
      "type": "BOOL",
      "direction": "input"
    },
    {
      "name": "MotorFeedback",
      "type": "BOOL",
      "direction": "input"
    },
    {
      "name": "OverloadOK",
      "type": "BOOL",
      "direction": "input"
    },
    {
      "name": "MotorContactor",
      "type": "BOOL",
      "direction": "output"
    },
    {
      "name": "RunLamp",
      "type": "BOOL",
      "direction": "output"
    },
    {
      "name": "Elapsed",
      "type": "TIME",
      "direction": "local"
    }
  ],
  "rungs": [
    {
      "id": 1,
      "network": {
        "type": "series",
        "id": 2,
        "items": [
          {
            "type": "parallel",
            "id": 3,
            "branches": [
              {
                "type": "contact",
                "id": 4,
                "contact": "normallyOpen",
                "variable": "StartPB"
              },
              {
                "type": "contact",
                "id": 5,
                "contact": "normallyOpen",
                "variable": "MotorContactor"
              }
            ]
          },
          {
            "type": "contact",
            "id": 6,
            "contact": "normallyOpen",
            "variable": "StopPB"
          },
          {
            "type": "contact",
            "id": 7,
            "contact": "normallyOpen",
            "variable": "OverloadOK"
          }
        ]
      },
      "coils": [
        {
          "id": 8,
          "variable": "MotorContactor"
        }
      ]
    },
    {
      "id": 9,
      "network": {
        "type": "series",
        "id": 10,
        "items": [
          {
            "type": "contact",
            "id": 11,
            "contact": "normallyOpen",
            "variable": "MotorFeedback"
          },
          {
            "type": "block",
            "id": 12,
            "block": "TON",
            "instance": "LampDelay",
            "preset": {
              "constant": "T#300ms"
            },
            "output": "Elapsed"
          }
        ]
      },
      "coils": [
        {
          "id": 13,
          "variable": "RunLamp"
        }
      ]
    }
  ]
}"##
}

fn plant() -> LabSession {
    let scene: SceneDefinition = parse_json(SCENE).unwrap();
    let station = scene.reference_station.clone();
    let mut s = LabSession::new(
        scene,
        station,
        PlatformProfile::generic(),
        SessionConfig::default(),
    )
    .unwrap();
    s.download(ladder(), 1, None).unwrap();
    s.set_plc_switch(PlcModeRequest::Run).unwrap();
    s
}
fn signal(s: &LabSession, name: &str) -> lab_engine::SignalId {
    s.signals()
        .iter()
        .find(|d| d.path == format!("plc.{name}"))
        .unwrap_or_else(|| panic!("no signal {name}"))
        .id
}
fn power(s: &LabSession, element: u32) -> bool {
    s.ladder_state()
        .unwrap()
        .power
        .iter()
        .find(|(e, _)| *e == element)
        .unwrap_or_else(|| panic!("no power for {element}"))
        .1
}
fn shown(s: &LabSession, name: &str) -> SignalValue {
    let id = signal(s, name);
    s.ladder_state()
        .unwrap()
        .values
        .iter()
        .find(|(signal, _)| *signal == id)
        .unwrap_or_else(|| panic!("no value for {name}"))
        .1
}
fn press(s: &mut LabSession, component: &str, down: bool) {
    let action = if down {
        OperatorAction::Press {
            component: component.into(),
        }
    } else {
        OperatorAction::Release {
            component: component.into(),
        }
    };
    s.operate(action).unwrap();
}

#[test]
fn a_ladder_program_drives_the_real_motor_and_is_observed_one_cycle_at_a_time() {
    let mut s = plant();
    assert!(s.ladder_state().is_none(), "no cycle yet");
    s.step_cycle().unwrap();
    let state = s.ladder_state().unwrap();
    assert!(state.scanned);
    let first = state.cycle;
    // Stop is not pressed: power reaches the overload contact only through Start.
    assert!(!power(&s, 2) && !power(&s, 3) && !power(&s, 6));
    assert_eq!(shown(&s, "StopPB"), SignalValue::Bool(true));
    press(&mut s, "start", true);
    s.step_cycle().unwrap();
    s.step_cycle().unwrap();
    // Each cycle is observed on its own: the state follows the latest one.
    assert_eq!(s.ladder_state().unwrap().cycle, first + 2);
    assert!(power(&s, 4) && power(&s, 3) && power(&s, 6) && power(&s, 7) && power(&s, 2));
    assert_eq!(shown(&s, "MotorContactor"), SignalValue::Bool(true));
    press(&mut s, "start", false);
    for _ in 0..3 {
        s.step_cycle().unwrap();
    }
    // Held: Start is open, the contactor's own contact carries the power.
    assert!(!power(&s, 4) && power(&s, 5) && power(&s, 2));
    // The lamp timer counts while the feedback is on, and its elapsed time is shown.
    let elapsed = shown(&s, "LampDelay.ET");
    assert!(
        matches!(elapsed, SignalValue::Time(us) if us > 0),
        "{elapsed:?}"
    );
    for _ in 0..4 {
        s.step_cycle().unwrap();
    }
    assert!(power(&s, 12), "the TON passes power after 300 ms");
    assert_eq!(shown(&s, "RunLamp"), SignalValue::Bool(true));
    assert_eq!(shown(&s, "LampDelay.Q"), SignalValue::Bool(true));
    // Compiler-owned symbols are not shown as values.
    let state = s.ladder_state().unwrap();
    assert!(
        state
            .values
            .iter()
            .all(|(id, _)| !s.signals().get(*id).unwrap().label.contains('#'))
    );
    // Stop opens the circuit.
    press(&mut s, "stop", true);
    s.step_cycle().unwrap();
    s.step_cycle().unwrap();
    assert!(!power(&s, 6) && !power(&s, 2));
    assert_eq!(shown(&s, "MotorContactor"), SignalValue::Bool(false));
    // The ladder's hidden symbols are not student variables.
    let (_, rows) = s.variables(None, 0, 500);
    assert!(rows.iter().all(|r| !r.name.contains('#')), "{rows:?}");
}
