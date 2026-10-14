use plc_types::PlcTime;
use process_simulation::{TankActuators, TankFillingProcess};
fn cycle() -> PlcTime {
    PlcTime::from_micros(100_000)
}
const FILL: TankActuators = TankActuators {
    inlet_valve: true,
    drain_pump: false,
};
const DRAIN: TankActuators = TankActuators {
    inlet_valve: false,
    drain_pump: true,
};
const BOTH: TankActuators = TankActuators {
    inlet_valve: true,
    drain_pump: true,
};
#[test]
fn initial_state_is_empty_low_true_high_false_and_analog_zero() {
    let state = TankFillingProcess::default().snapshot();
    assert_eq!(state.level_permille(), 0);
    assert!(state.sensors().low_level);
    assert!(!state.sensors().high_level);
    assert_eq!(state.sensors().level_percent.get().to_bits(), 0);
    assert_eq!(state.actuators(), TankActuators::default());
}
#[test]
fn one_cycle_fills_one_hundred_permille() {
    let mut tank = TankFillingProcess::default();
    let s = tank.step(cycle(), FILL).unwrap();
    assert_eq!(s.level_permille(), 100);
    assert_eq!(s.sensors().level_percent.get(), 10.0);
    assert_eq!(s.actuators(), FILL);
}
#[test]
fn thresholds_switch_exactly_at_low_two_hundred_and_high_eight_hundred() {
    let mut tank = TankFillingProcess::default();
    for level in (100..=1000).step_by(100) {
        let s = tank.step(cycle(), FILL).unwrap();
        assert_eq!(s.level_permille(), level);
        assert_eq!(s.sensors().low_level, level <= 200);
        assert_eq!(s.sensors().high_level, level >= 800);
        assert_eq!(s.sensors().level_percent.get(), f32::from(level) / 10.0);
    }
}
#[test]
fn fill_clamps_at_one_thousand() {
    let mut tank = TankFillingProcess::default();
    for _ in 0..20 {
        tank.step(cycle(), FILL).unwrap();
    }
    assert_eq!(tank.snapshot().level_permille(), 1000);
}
#[test]
fn one_drain_cycle_from_full_produces_nine_twenty_then_clamps_at_zero() {
    let mut tank = TankFillingProcess::default();
    tank.step(PlcTime::from_micros(1_000_000), FILL).unwrap();
    assert_eq!(tank.step(cycle(), DRAIN).unwrap().level_permille(), 920);
    for _ in 0..20 {
        tank.step(cycle(), DRAIN).unwrap();
    }
    assert_eq!(tank.snapshot().level_permille(), 0);
    assert!(tank.snapshot().sensors().low_level);
}
#[test]
fn simultaneous_actuators_apply_net_contribution_before_clamping() {
    let mut tank = TankFillingProcess::default();
    assert_eq!(tank.step(cycle(), BOTH).unwrap().level_permille(), 20);
    tank.step(PlcTime::from_micros(1_000_000), FILL).unwrap();
    assert_eq!(tank.step(cycle(), BOTH).unwrap().level_permille(), 1000);
}
#[test]
fn neither_actuator_preserves_level_and_records_stopped_commands() {
    let mut tank = TankFillingProcess::default();
    tank.step(cycle(), FILL).unwrap();
    let s = tank.step(cycle(), TankActuators::default()).unwrap();
    assert_eq!(s.level_permille(), 100);
    assert_eq!(s.actuators(), TankActuators::default());
}
#[test]
fn reset_returns_exact_initial_snapshot() {
    let mut tank = TankFillingProcess::default();
    let initial = tank.snapshot();
    tank.step(cycle(), BOTH).unwrap();
    assert_eq!(tank.reset(), initial);
}
#[test]
fn same_duration_and_actuator_sequence_produces_identical_snapshots() {
    let sequence = [
        (100_000, FILL),
        (100_000, BOTH),
        (1250, BOTH),
        (1_000_000, DRAIN),
        (99, FILL),
        (u64::MAX, FILL),
    ];
    let run = || {
        let mut tank = TankFillingProcess::default();
        sequence
            .iter()
            .map(|(dt, a)| tank.step(PlcTime::from_micros(*dt), *a).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
#[test]
fn fractional_contributions_truncate_independently_without_hidden_remainder() {
    let mut tank = TankFillingProcess::default();
    for _ in 0..100 {
        tank.step(PlcTime::from_micros(999), FILL).unwrap();
    }
    assert_eq!(tank.snapshot().level_permille(), 0);
    assert_eq!(
        tank.step(PlcTime::from_micros(1250), BOTH)
            .unwrap()
            .level_permille(),
        0
    );
    assert_eq!(
        tank.step(PlcTime::from_micros(1000), FILL)
            .unwrap()
            .level_permille(),
        1
    );
}
#[test]
fn maximum_u64_duration_cannot_wrap_and_is_clamped_deterministically() {
    let mut tank = TankFillingProcess::default();
    let max = PlcTime::from_micros(u64::MAX);
    assert_eq!(tank.step(max, FILL).unwrap().level_permille(), 1000);
    assert_eq!(tank.step(max, DRAIN).unwrap().level_permille(), 0);
    assert_eq!(tank.step(max, BOTH).unwrap().level_permille(), 1000);
    assert_eq!(
        tank.step(PlcTime::ZERO, DRAIN).unwrap().level_permille(),
        1000
    );
}
#[test]
fn initialization_records_committed_actuators_without_integrating_a_step() {
    let tank = TankFillingProcess::new(BOTH);
    assert_eq!(tank.snapshot().level_permille(), 0);
    assert_eq!(tank.snapshot().actuators(), BOTH);
}
#[test]
fn analog_fraction_uses_finite_binary32_without_becoming_integrated_state() {
    let mut tank = TankFillingProcess::default();
    let s = tank.step(PlcTime::from_micros(33_000), FILL).unwrap();
    assert_eq!(s.level_permille(), 33);
    assert_eq!(s.sensors().level_percent.get().to_bits(), 3.3_f32.to_bits());
    assert_eq!(tank.step(cycle(), FILL).unwrap().level_permille(), 133);
}
