#[allow(dead_code)]
#[path = "../../plc-ir/tests/support/mod.rs"]
mod support;
use plc_ir::{CallBinding, Program, validate};
use plc_runtime::{
    FunctionBlockStateSnapshot, Runtime, RuntimeConfig, RuntimeSnapshot, RuntimeState, lower,
};
use plc_types::{
    PlcTime, PlcType, PlcValue,
    compatibility::BinaryOperator as B,
    diagnostics::{RelatedIdentity, codes},
    function_blocks::{FunctionBlockInstance, TonParameter},
    identity::SymbolId,
};
use support::ton::*;

fn runtime(program: Program, period: PlcTime, budget: u64) -> Runtime {
    Runtime::new(
        lower(&validate(&program).unwrap()).unwrap(),
        RuntimeConfig::new(period, budget).unwrap(),
    )
}
fn associated() -> (Runtime, [SymbolId; 4]) {
    let mut f = TonFixture::new();
    let pins = [f.enable, f.preset, f.q, f.et];
    let call = f.associated_call();
    (runtime(f.finish(vec![call]), ms(100), 100), pins)
}
fn scan(r: &mut Runtime, pins: [SymbolId; 4], input: bool, preset: u64) -> RuntimeSnapshot {
    r.set_input(pins[0], PlcValue::Bool(input)).unwrap();
    r.set_input(pins[1], PlcValue::Time(ms(preset))).unwrap();
    r.single_scan().unwrap().snapshot().clone()
}
fn state(s: &RuntimeSnapshot, n: u32) -> (bool, PlcTime) {
    let FunctionBlockStateSnapshot::Ton { q, et } = s.function_blocks()[&iid(n)];
    (q, et)
}
fn assert_timer(s: &RuntimeSnapshot, pins: [SymbolId; 4], q: bool, et: u64) {
    assert_eq!(state(s, 11), (q, ms(et)));
    assert_eq!(s.outputs()[&pins[2]], PlcValue::Bool(q));
    assert_eq!(s.outputs()[&pins[3]], PlcValue::Time(ms(et)));
}
#[test]
fn false_input_is_inactive_and_zero_preset_is_immediate() {
    let (mut r, pins) = associated();
    assert_timer(&scan(&mut r, pins, false, 0), pins, false, 0);
    assert_timer(&scan(&mut r, pins, true, 0), pins, true, 0);
}
#[test]
fn exact_100ms_cycle_300ms_preset_uses_scan_start_and_caps_elapsed() {
    let (mut r, pins) = associated();
    let mut actual = Vec::new();
    for (index, et, q) in [
        (0, 0, false),
        (1, 100, false),
        (2, 200, false),
        (3, 300, true),
        (4, 300, true),
        (5, 300, true),
    ] {
        let start = r.snapshot().time();
        let s = scan(&mut r, pins, true, 300);
        assert_timer(&s, pins, q, et);
        assert_eq!(start, ms(index * 100));
        assert_eq!(s.time(), ms((index + 1) * 100));
        assert_eq!(s.completed_scans(), index + 1);
        actual.push((
            start.as_micros() / 1_000,
            et,
            q,
            s.time().as_micros() / 1_000,
        ));
    }
    println!("TON timeline (start ms, ET ms, Q, end ms): {actual:?}");
}
#[test]
fn false_after_expiration_resets_and_true_starts_a_new_epoch() {
    let (mut r, pins) = associated();
    for _ in 0..4 {
        scan(&mut r, pins, true, 300);
    }
    assert_timer(&scan(&mut r, pins, false, 300), pins, false, 0);
    assert_timer(&scan(&mut r, pins, true, 300), pins, false, 0);
    assert_timer(&scan(&mut r, pins, true, 300), pins, false, 100);
}
#[test]
fn preset_changes_recalculate_against_the_retained_epoch() {
    let (mut r, pins) = associated();
    assert_timer(&scan(&mut r, pins, true, 500), pins, false, 0);
    assert_timer(&scan(&mut r, pins, true, 50), pins, true, 50);
    assert_timer(&scan(&mut r, pins, true, 400), pins, false, 200);
    assert_timer(&scan(&mut r, pins, true, 250), pins, true, 250);
}
#[test]
fn q_and_et_associations_are_visible_to_later_same_scan_instructions() {
    let mut f = TonFixture::new();
    let (q, et, seen_q, seen_et) = (f.q, f.et, f.seen_q, f.seen_et);
    let call = f.associated_call();
    let read_q = f.f.read(q);
    let read_et = f.f.read(et);
    let copy_q = f.f.assign(seen_q, read_q);
    let copy_et = f.f.assign(seen_et, read_et);
    let enable = f.enable;
    let preset = f.preset;
    let mut r = runtime(f.finish(vec![call, copy_q, copy_et]), ms(100), 9);
    r.set_input(enable, PlcValue::Bool(true)).unwrap();
    r.set_input(preset, PlcValue::Time(ms(100))).unwrap();
    for (expected_q, expected_et) in [(false, 0), (true, 100)] {
        let result = r.single_scan().unwrap();
        assert_eq!(result.operations(), 9);
        let s = result.snapshot();
        assert_eq!(s.outputs()[&seen_q], PlcValue::Bool(expected_q));
        assert_eq!(s.outputs()[&seen_et], PlcValue::Time(ms(expected_et)));
    }
}
#[test]
fn omitted_associations_update_instance_outputs_without_touching_other_symbols() {
    for (associate_q, associate_et, expected_cost) in [
        (false, false, 3),
        (true, false, 4),
        (false, true, 4),
        (true, true, 5),
    ] {
        let mut f = TonFixture::new();
        let (enable, preset, q, et) = (f.enable, f.preset, f.q, f.et);
        let bindings = f.bindings(associate_q.then_some(q), associate_et.then_some(et));
        let call = f.call(iid(11), bindings);
        let mut r = runtime(f.finish(vec![call]), ms(100), expected_cost);
        r.set_input(enable, PlcValue::Bool(true)).unwrap();
        r.set_input(preset, PlcValue::Time(ms(100))).unwrap();
        for _ in 0..2 {
            assert_eq!(r.single_scan().unwrap().operations(), expected_cost);
        }
        let s = r.snapshot();
        assert_eq!(state(&s, 11), (true, ms(100)));
        assert_eq!(s.outputs()[&q], PlcValue::Bool(associate_q));
        assert_eq!(
            s.outputs()[&et],
            PlcValue::Time(if associate_et { ms(100) } else { PlcTime::ZERO })
        );
        assert_eq!(s.committed().len() + s.staged_inputs().len(), 9); // No generated timer symbols.
    }
}
#[test]
fn repeated_calls_see_sequential_working_state_at_the_same_now() {
    let mut f = TonFixture::new();
    let (enable, preset, q, et, first_q, first_et) =
        (f.enable, f.preset, f.q, f.et, f.seen_q, f.seen_et);
    let first = f.constant_call(true, PlcTime::ZERO, Some(first_q), Some(first_et));
    let reset = f.constant_call(false, ms(300), None, None);
    let last = f.associated_call();
    let mut r = runtime(f.finish(vec![first, reset, last]), ms(100), 13);
    r.set_input(enable, PlcValue::Bool(true)).unwrap();
    r.set_input(preset, PlcValue::Time(ms(300))).unwrap();
    for _ in 0..4 {
        let result = r.single_scan().unwrap();
        assert_eq!(result.operations(), 13);
        assert_eq!(state(result.snapshot(), 11), (false, PlcTime::ZERO));
        assert_eq!(result.snapshot().outputs()[&first_q], PlcValue::Bool(true));
        assert_eq!(
            result.snapshot().outputs()[&first_et],
            PlcValue::Time(PlcTime::ZERO)
        );
        assert_eq!(result.snapshot().outputs()[&q], PlcValue::Bool(false));
        assert_eq!(
            result.snapshot().outputs()[&et],
            PlcValue::Time(PlcTime::ZERO)
        );
    }
}
#[test]
fn exactly_two_calls_to_one_instance_are_allowed_and_later_reset_wins() {
    let mut f = TonFixture::new();
    let (first_q, q) = (f.seen_q, f.q);
    let first = f.constant_call(true, PlcTime::ZERO, Some(first_q), Some(f.seen_et));
    let second = f.constant_call(false, PlcTime::ZERO, Some(q), Some(f.et));
    let mut r = runtime(f.finish(vec![first, second]), ms(100), 10);
    for _ in 0..3 {
        let scan = r.single_scan().unwrap();
        assert_eq!(scan.operations(), 10);
        assert_eq!(scan.snapshot().outputs()[&first_q], PlcValue::Bool(true));
        assert_eq!(scan.snapshot().outputs()[&q], PlcValue::Bool(false));
        assert_eq!(state(scan.snapshot(), 11), (false, PlcTime::ZERO));
    }
}
#[test]
fn skipped_calls_preserve_state_and_associations_then_catch_elapsed_virtual_time() {
    let mut f = TonFixture::new();
    let (enable, preset, invoke, q, et) = (f.enable, f.preset, f.invoke, f.q, f.et);
    let call = f.associated_call();
    let condition = f.f.read(invoke);
    let branch = f.f.conditional(condition, vec![call], vec![]);
    let mut r = runtime(f.finish(vec![branch]), ms(100), 7);
    r.set_input(enable, PlcValue::Bool(true)).unwrap();
    r.set_input(preset, PlcValue::Time(ms(300))).unwrap();
    assert_eq!(r.single_scan().unwrap().operations(), 2);
    assert_eq!(state(&r.snapshot(), 11), (false, PlcTime::ZERO));
    r.set_input(invoke, PlcValue::Bool(true)).unwrap();
    r.single_scan().unwrap(); // Activation at 100 ms, not 0 ms.
    r.single_scan().unwrap(); // ET 100 at 200 ms.
    let before = r.snapshot();
    r.set_input(invoke, PlcValue::Bool(false)).unwrap();
    r.set_input(enable, PlcValue::Bool(false)).unwrap();
    for _ in 0..2 {
        assert_eq!(r.single_scan().unwrap().operations(), 2);
        assert_eq!(r.snapshot().function_blocks(), before.function_blocks());
        assert_eq!(r.snapshot().outputs(), before.outputs());
    }
    r.set_input(enable, PlcValue::Bool(true)).unwrap();
    r.set_input(invoke, PlcValue::Bool(true)).unwrap();
    r.single_scan().unwrap(); // 500 ms minus retained epoch 100 ms.
    assert_timer(&r.snapshot(), [enable, preset, q, et], true, 300);
}
#[test]
fn two_instances_of_one_definition_have_independent_state() {
    let mut f = TonFixture::new();
    let (enable, preset) = (f.enable, f.preset);
    f.catalog
        .declare_instance(
            FunctionBlockInstance::new(iid(12), did(77), support::entry(), "OtherTimer", None)
                .unwrap(),
        )
        .unwrap();
    let first = f.associated_call();
    let operand = f.f.read(enable);
    let inverted = f.f.not(operand);
    let second_bindings = vec![
        CallBinding::input(TonParameter::In.id(), inverted),
        CallBinding::input(TonParameter::Pt.id(), f.f.read(preset)),
    ];
    let second = f.call(iid(12), second_bindings);
    let mut r = runtime(f.finish(vec![first, second]), ms(100), 100);
    r.set_input(enable, PlcValue::Bool(true)).unwrap();
    r.set_input(preset, PlcValue::Time(ms(100))).unwrap();
    for _ in 0..2 {
        r.single_scan().unwrap();
    }
    assert_eq!(state(&r.snapshot(), 11), (true, ms(100)));
    assert_eq!(state(&r.snapshot(), 12), (false, PlcTime::ZERO));
    r.set_input(enable, PlcValue::Bool(false)).unwrap();
    r.single_scan().unwrap();
    assert_eq!(state(&r.snapshot(), 11), (false, PlcTime::ZERO));
    assert_eq!(state(&r.snapshot(), 12), (false, PlcTime::ZERO));
    r.single_scan().unwrap();
    assert_eq!(state(&r.snapshot(), 12), (true, ms(100)));
}
#[test]
fn reset_restores_every_instance_and_all_m1b_state_defaults() {
    let (mut r, pins) = associated();
    let initial = r.snapshot();
    for _ in 0..4 {
        scan(&mut r, pins, true, 300);
    }
    let old_snapshot = r.snapshot();
    r.reset();
    assert_eq!(r.snapshot(), initial);
    assert_eq!(state(&old_snapshot, 11), (true, ms(300))); // Observation is owned, not a mutable handle.
    assert_timer(&scan(&mut r, pins, true, 300), pins, false, 0);
}
#[test]
fn identical_input_sequences_produce_identical_results_and_snapshots() {
    let (mut a, pins) = associated();
    let (mut b, other_pins) = associated();
    for (enable, preset) in [
        (false, 300),
        (true, 300),
        (true, 300),
        (true, 50),
        (true, 500),
        (true, 500),
        (false, 500),
        (true, 0),
    ] {
        assert_eq!(
            scan(&mut a, pins, enable, preset),
            scan(&mut b, other_pins, enable, preset)
        );
    }
}
#[test]
fn shuffled_binding_order_cannot_change_input_or_output_charge_order() {
    let mut f = TonFixture::new();
    let mut bindings = f.bindings(Some(f.q), Some(f.et));
    bindings.reverse();
    let pt_source = match &bindings[2] {
        CallBinding::Input { value, .. } => value.source(),
        _ => panic!("PT expected"),
    };
    let et = f.et;
    let statement = f.call(iid(11), bindings);
    let call_source = statement.source();
    let p = f.finish(vec![statement]);
    let mut input_limited = runtime(p.clone(), ms(100), 1);
    let error = input_limited.single_scan().unwrap_err();
    assert_eq!(
        error.diagnostic().unwrap().source().unwrap().reference(),
        pt_source
    );
    let mut output_limited = runtime(p, ms(100), 4);
    let error = output_limited.single_scan().unwrap_err();
    let d = error.diagnostic().unwrap();
    assert_eq!(d.code(), codes::EXECUTION_BUDGET);
    assert_eq!(d.source().unwrap().reference(), call_source);
    assert_eq!(d.related(), Some(RelatedIdentity::Symbol(et))); // Q was charged before ET.
    assert_eq!(
        state(&output_limited.snapshot(), 11),
        (false, PlcTime::ZERO)
    );
    assert_eq!(output_limited.snapshot().completed_scans(), 0);
    assert_eq!(output_limited.state(), RuntimeState::Faulted);
}
#[test]
fn ton_expressions_use_normal_expression_charges_and_fault_sources() {
    let mut f = TonFixture::new();
    let one = f.f.constant(PlcValue::Int(1));
    let zero = f.f.constant(PlcValue::Int(0));
    let division = f.f.binary(B::Divide, PlcType::Int, one, zero);
    let fault_source = division.source();
    let zero = f.f.constant(PlcValue::Int(0));
    let input = f.f.binary(B::Greater, PlcType::Bool, division, zero);
    let preset = f.f.constant(PlcValue::Time(ms(300)));
    let call = f.call(
        iid(11),
        vec![
            CallBinding::input(TonParameter::Pt.id(), preset),
            CallBinding::input(TonParameter::In.id(), input),
        ],
    );
    let mut r = runtime(f.finish(vec![call]), ms(100), 3);
    let error = r.single_scan().unwrap_err();
    assert_eq!(error.diagnostic().unwrap().code(), codes::DIVISION_BY_ZERO);
    assert_eq!(
        error.diagnostic().unwrap().source().unwrap().reference(),
        fault_source
    );
    assert_eq!(state(&r.snapshot(), 11), (false, PlcTime::ZERO));
}
