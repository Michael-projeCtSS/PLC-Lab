//! The electrically heated vessel (ADR-0047): two lags after a dead time on a fixed
//! grid of world time, an analog power input, a door disturbance and a built-in
//! temperature transmitter.
use lab_model::{scene::SceneDefinition, validate_scene};
use process_simulation::{AnalogUnit, Quantity, SceneRuntime};
use simulation_core::WorldTime;

/// Ambient 20 °C, 0.8 °C per % of power (100 % gives +80 °C), element lag 5 s, vessel
/// lag 30 s, dead time 2 s; an open door loses half the vessel's rise again; the
/// transmitter measures 0..150 °C as 4-20 mA.
fn scene() -> SceneDefinition {
    let scene: SceneDefinition = serde_json::from_str(
        r#"{
        "format": "plclab-scene", "schemaVersion": 1, "id": "test.oven", "revision": 1,
        "title": "Oven", "summary": "Test",
        "components": [
          {"id": "oven", "label": "Oven", "kind": {"type": "heatedVessel", "ambientC": 20,
            "gainMilliCPerPercent": 800, "elementLagMs": 5000, "vesselLagMs": 30000,
            "deadTimeMs": 2000, "doorLossPermille": 500, "power": "current4To20mA",
            "rangeMinC": 0, "rangeMaxC": 150, "signal": "current4To20mA"}}
        ],
        "couplings": [],
        "installation": {"cpu": "cpu.compact", "modules": [], "wiring": []},
        "referenceStation": {"name": "S", "cpu": "cpu.compact", "modules": [], "ioMap": []}
      }"#,
    )
    .unwrap();
    validate_scene(&scene).unwrap();
    scene
}

fn at(ms: u64) -> WorldTime {
    WorldTime::from_micros(ms * 1000)
}
fn int(s: &SceneRuntime, name: &str) -> i64 {
    match s.quantity(0, name).unwrap() {
        Quantity::Integer(i) => i,
        Quantity::Bool(b) => i64::from(b),
    }
}
/// Heater power in per mille as a 4-20 mA current.
fn power(s: &mut SceneRuntime, permille: i64) {
    s.set_analog_input(0, "power", AnalogUnit::Microamp, 4_000 + permille * 16)
        .unwrap();
}

#[test]
fn full_power_settles_at_ambient_plus_gain_after_the_lags() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    assert_eq!(int(&s, "temperatureMilliC"), 20_000);
    power(&mut s, 1000);
    assert_eq!(int(&s, "powerPermille"), 1000);
    // Nothing reaches the element during the dead time.
    s.advance_to(at(2000)).unwrap();
    assert_eq!(int(&s, "elementMilliC"), 20_000);
    s.advance_to(at(2100)).unwrap();
    assert!(int(&s, "elementMilliC") > 20_000);
    // Two lags: slow at first, then most of the way after a few vessel time constants.
    s.advance_to(at(32_000)).unwrap();
    let after_30s = int(&s, "temperatureMilliC");
    assert!((40_000..65_000).contains(&after_30s), "{after_30s}");
    s.advance_to(at(400_000)).unwrap();
    let settled = int(&s, "temperatureMilliC");
    assert!((settled - 100_000).abs() <= 100, "{settled}");
}

#[test]
fn the_state_does_not_depend_on_how_time_is_partitioned() {
    let mut one = SceneRuntime::new(&scene(), 1).unwrap();
    let mut many = SceneRuntime::new(&scene(), 1).unwrap();
    for s in [&mut one, &mut many] {
        power(s, 650);
    }
    one.advance_to(at(61_234)).unwrap();
    let mut t = 0;
    for step in [1u64, 7, 13, 100, 3, 999, 2_500].iter().cycle() {
        t = (t + step).min(61_234);
        many.advance_to(at(t)).unwrap();
        if t == 61_234 {
            break;
        }
    }
    for name in ["temperatureMilliC", "elementMilliC", "signal"] {
        assert_eq!(int(&one, name), int(&many, name), "{name}");
    }
}

#[test]
fn an_open_door_is_a_disturbance_that_lowers_the_temperature() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    power(&mut s, 500);
    s.advance_to(at(400_000)).unwrap();
    // 50 % gives +40 °C.
    assert!((int(&s, "temperatureMilliC") - 60_000).abs() <= 100);
    s.set_door("oven", true).unwrap();
    assert_eq!(int(&s, "doorOpen"), 1);
    s.advance_to(at(800_000)).unwrap();
    // With the door open the vessel loses half its rise again: +40 / 1.5 = +26.7 °C.
    assert!((int(&s, "temperatureMilliC") - 46_667).abs() <= 100);
    s.set_door("oven", false).unwrap();
    s.advance_to(at(1_200_000)).unwrap();
    assert!((int(&s, "temperatureMilliC") - 60_000).abs() <= 100);
}

#[test]
fn the_power_input_saturates_and_ignores_the_wrong_kind_of_signal() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    s.set_analog_input(0, "power", AnalogUnit::Microamp, 23_000)
        .unwrap();
    assert_eq!(int(&s, "powerPermille"), 1000, "above 20 mA is full power");
    s.set_analog_input(0, "power", AnalogUnit::Microamp, 2_000)
        .unwrap();
    assert_eq!(int(&s, "powerPermille"), 0, "below 4 mA is no power");
    s.set_analog_input(0, "power", AnalogUnit::Millivolt, 10_000)
        .unwrap();
    assert_eq!(int(&s, "powerPermille"), 0, "a voltage is no current");
    assert!(
        s.set_analog_input(0, "temperature", AnalogUnit::Microamp, 1)
            .is_err()
    );
    assert!(s.is_analog_sink(0, "power"));
    assert!(!s.is_analog_sink(0, "temperature"));
}

#[test]
fn the_transmitter_reports_the_temperature_over_its_range() {
    let mut s = SceneRuntime::new(&scene(), 1).unwrap();
    // 20 °C of 0..150 °C: 4 + 16 * 20/150 = 6.133 mA.
    let field = s.analog_output(0, "temperature").unwrap();
    assert_eq!((field.unit, field.value), (AnalogUnit::Microamp, 6_133));
    power(&mut s, 1000);
    s.advance_to(at(400_000)).unwrap();
    // 100 °C: 4 + 16 * 100/150 = 14.667 mA.
    let field = s.analog_output(0, "temperature").unwrap();
    assert!((field.value - 14_667).abs() <= 2, "{}", field.value);
    assert_eq!(int(&s, "signal"), field.value);
}

#[test]
fn identical_inputs_give_identical_runs() {
    let run = || {
        let mut s = SceneRuntime::new(&scene(), 7).unwrap();
        let mut trace = Vec::new();
        for k in 0..300u64 {
            power(&mut s, i64::try_from((k * 37) % 1001).unwrap());
            if k == 150 {
                s.set_door("oven", true).unwrap();
            }
            s.advance_to(at(k * 100)).unwrap();
            trace.push(int(&s, "temperatureMilliC"));
        }
        trace
    };
    assert_eq!(run(), run());
}
