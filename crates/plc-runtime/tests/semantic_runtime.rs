// Reuse the small direct-IR assembler strictly in test scope.
#[path = "../../plc-ir/tests/support/mod.rs"]
mod support;
use plc_ir::{Expression, Program, validate};
use plc_runtime::{
    ConfigurationError, InputError, LoweringError, MemoryRole, Runtime, RuntimeConfig,
    RuntimeSnapshot, RuntimeState, ScanError, lower,
};
use plc_types::{
    PlcTime, PlcType, PlcValue,
    compatibility::{BinaryOperator as B, UnaryOperator as U},
    diagnostics::{DiagnosticCategory, RelatedIdentity, Severity},
    identity::PouId,
    symbols::{
        Pou, PouKind, Scope, Symbol,
        VariableCategory::{InOut, Input, Local, Output},
    },
};
use support::*;

fn config(period: u64, budget: u64) -> RuntimeConfig {
    RuntimeConfig::new(PlcTime::from_micros(period), budget).unwrap()
}
fn runtime(program: Program, period: u64, budget: u64) -> Runtime {
    let plan = lower(&validate(&program).unwrap()).unwrap();
    Runtime::new(plan, config(period, budget))
    // `program` is dropped: Runtime cannot retain its lifetime.
}
fn value(snapshot: &RuntimeSnapshot, id: plc_types::identity::SymbolId) -> PlcValue {
    snapshot.committed()[&id]
}
fn fault_code(error: &ScanError) -> &str {
    assert!(matches!(error, ScanError::Fault(_)));
    let d = error.diagnostic().unwrap();
    assert_eq!(d.category(), DiagnosticCategory::RuntimeFault);
    assert_eq!(d.severity(), Severity::Error);
    d.code()
}
fn binary_runtime(
    op: B,
    a: PlcValue,
    b: PlcValue,
    t: PlcType,
) -> (Runtime, plc_types::identity::SymbolId) {
    let mut f = Fixture::new();
    let target = f.declare("Result", t, Output);
    let left = f.constant(a);
    let right = f.constant(b);
    let expression = f.binary(op, t, left, right);
    let assignment = f.assign(target, expression);
    (runtime(f.finish(vec![assignment]), 100, 100), target)
}
fn unary_runtime(op: U, a: PlcValue) -> (Runtime, plc_types::identity::SymbolId) {
    let mut f = Fixture::new();
    let target = f.declare("Result", a.plc_type(), Output);
    let operand = f.constant(a);
    let expression = Expression::unary(f.anchor(), a.plc_type(), op, operand);
    let assignment = f.assign(target, expression);
    (runtime(f.finish(vec![assignment]), 100, 100), target)
}

#[test]
fn golden_motor_assign_start_runs_the_complete_owned_path() {
    let mut f = Fixture::new();
    let start = f.declare("Start", PlcType::Bool, Input);
    let motor = f.declare("Motor", PlcType::Bool, Output);
    let expression = f.read(start);
    let assignment = f.assign(motor, expression);
    let mut r = runtime(f.finish(vec![assignment]), 1_000, 2);
    for (index, input) in [false, true, false].into_iter().enumerate() {
        r.set_input(start, PlcValue::Bool(input)).unwrap();
        let scan = r.single_scan().unwrap();
        assert_eq!(scan.snapshot().outputs()[&motor], PlcValue::Bool(input));
        assert_eq!(scan.operations(), 2);
        assert_eq!(scan.snapshot().completed_scans(), index as u64 + 1);
        assert_eq!(
            scan.snapshot().time().as_micros(),
            (index as u64 + 1) * 1_000
        );
    }
}
#[test]
fn golden_motor_start_and_not_stop_covers_the_boolean_truth_table() {
    let mut f = Fixture::new();
    let start = f.declare("Start", PlcType::Bool, Input);
    let stop = f.declare("Stop", PlcType::Bool, Input);
    let motor = f.declare("Motor", PlcType::Bool, Output);
    let left = f.read(start);
    let stop_value = f.read(stop);
    let right = f.not(stop_value);
    let expression = f.binary(B::And, PlcType::Bool, left, right);
    let assignment = f.assign(motor, expression);
    let mut r = runtime(f.finish(vec![assignment]), 10, 5);
    for (start_value, stop_value, expected) in [
        (false, false, false),
        (true, false, true),
        (true, true, false),
        (false, true, false),
    ] {
        r.set_input(start, PlcValue::Bool(start_value)).unwrap();
        r.set_input(stop, PlcValue::Bool(stop_value)).unwrap();
        let scan = r.single_scan().unwrap();
        assert_eq!(scan.snapshot().outputs()[&motor], PlcValue::Bool(expected));
        assert_eq!(scan.operations(), 5);
    }
}
#[test]
fn golden_level_high_pump_false_executes_only_the_selected_body() {
    let mut f = Fixture::new();
    let high = f.declare("LevelHigh", PlcType::Bool, Input);
    let pump = f.declare("Pump", PlcType::Bool, Output);
    let condition = f.read(high);
    let off = f.constant(PlcValue::Bool(false));
    let assignment = f.assign(pump, off);
    let branch = f.conditional(condition, vec![assignment], vec![]);
    let mut r = runtime(f.finish(vec![branch]), 10, 4);
    for (input, operations) in [(false, 2), (true, 4)] {
        r.set_input(high, PlcValue::Bool(input)).unwrap();
        let scan = r.single_scan().unwrap();
        assert_eq!(scan.snapshot().outputs()[&pump], PlcValue::Bool(false));
        assert_eq!(scan.operations(), operations);
    }
}
#[test]
fn golden_value_limit_alarm_executes_both_outcomes() {
    let mut f = Fixture::new();
    let input = f.declare("Value", PlcType::Int, Input);
    let limit = f.declare("Limit", PlcType::Int, Input);
    let alarm = f.declare("Alarm", PlcType::Bool, Output);
    let a = f.read(input);
    let b = f.read(limit);
    let condition = f.binary(B::Greater, PlcType::Bool, a, b);
    let yes = f.constant(PlcValue::Bool(true));
    let no = f.constant(PlcValue::Bool(false));
    let then_write = f.assign(alarm, yes);
    let else_write = f.assign(alarm, no);
    let branch = f.conditional(condition, vec![then_write], vec![else_write]);
    let mut r = runtime(f.finish(vec![branch]), 20, 6);
    for (v, l, expected) in [(11, 10, true), (10, 10, false), (-1, 10, false)] {
        r.set_input(input, PlcValue::Int(v)).unwrap();
        r.set_input(limit, PlcValue::Int(l)).unwrap();
        let scan = r.single_scan().unwrap();
        assert_eq!(scan.snapshot().outputs()[&alarm], PlcValue::Bool(expected));
        assert_eq!(scan.operations(), 6);
    }
}

#[test]
fn staged_inputs_are_distinct_from_the_stable_scan_image() {
    let mut f = Fixture::new();
    let input = f.declare("InputA", PlcType::Bool, Input);
    let a = f.declare("A", PlcType::Bool, Output);
    let b = f.declare("B", PlcType::Bool, Output);
    let read_a = f.read(input);
    let first = f.assign(a, read_a);
    let read_b = f.read(input);
    let second = f.assign(b, read_b);
    let mut r = runtime(f.finish(vec![first, second]), 100, 4);
    r.set_input(input, PlcValue::Bool(true)).unwrap();
    assert_eq!(r.snapshot().staged_inputs()[&input], PlcValue::Bool(true));
    assert_eq!(r.snapshot().input_image()[&input], PlcValue::Bool(false));
    let scan = r.single_scan().unwrap();
    assert_eq!(scan.snapshot().input_image()[&input], PlcValue::Bool(true));
    assert_eq!(value(scan.snapshot(), a), PlcValue::Bool(true));
    assert_eq!(value(scan.snapshot(), b), PlcValue::Bool(true));
    r.set_input(input, PlcValue::Bool(false)).unwrap();
    assert_eq!(r.snapshot().input_image()[&input], PlcValue::Bool(true));
    assert_eq!(value(&r.snapshot(), a), PlcValue::Bool(true));
    let scan = r.single_scan().unwrap();
    assert_eq!(value(scan.snapshot(), a), PlcValue::Bool(false));
    assert_eq!(value(scan.snapshot(), b), PlcValue::Bool(false));
}
#[test]
fn internal_and_output_writes_are_visible_later_in_the_same_scan_and_persist() {
    let mut f = Fixture::new();
    let input = f.declare("InputA", PlcType::Bool, Input);
    let internal = f.declare("InternalA", PlcType::Bool, Local);
    let a = f.declare("OutputA", PlcType::Bool, Output);
    let b = f.declare("OutputB", PlcType::Bool, Output);
    let v = f.read(input);
    let write_internal = f.assign(internal, v);
    let v = f.read(internal);
    let write_a = f.assign(a, v);
    let v = f.read(a);
    let write_b = f.assign(b, v);
    let mut r = runtime(f.finish(vec![write_internal, write_a, write_b]), 5, 6);
    r.set_input(input, PlcValue::Bool(true)).unwrap();
    let scan = r.single_scan().unwrap();
    for id in [internal, a, b] {
        assert_eq!(value(scan.snapshot(), id), PlcValue::Bool(true));
    }
    assert!(!scan.snapshot().outputs().contains_key(&internal));
    assert_eq!(r.snapshot(), *scan.snapshot());
}

// A success initializes non-default committed values; a later failure follows two working writes.
fn transactional_program(
    budget_failure: bool,
) -> (
    Program,
    plc_types::identity::SymbolId,
    plc_types::identity::SymbolId,
    plc_types::identity::SymbolId,
) {
    let mut f = Fixture::new();
    let fail = f.declare("Fail", PlcType::Bool, Input);
    let output = f.declare("Output", PlcType::Bool, Output);
    let internal = f.declare("Internal", PlcType::Bool, Local);
    let number = f.declare("Number", PlcType::Int, Output);
    let no = f.constant(PlcValue::Bool(false));
    let output_off = f.assign(output, no.clone());
    let internal_off = f.assign(internal, no.clone());
    let final_write = if budget_failure {
        f.assign(output, no)
    } else {
        let one = f.constant(PlcValue::Int(1));
        let zero = f.constant(PlcValue::Int(0));
        let division = f.binary(B::Divide, PlcType::Int, one, zero);
        f.assign(number, division)
    };
    let yes = f.constant(PlcValue::Bool(true));
    let output_on = f.assign(output, yes.clone());
    let internal_on = f.assign(internal, yes);
    let condition = f.read(fail);
    let branch = f.conditional(
        condition,
        vec![output_off, internal_off, final_write],
        vec![output_on, internal_on],
    );
    (f.finish(vec![branch]), fail, output, internal)
}
#[test]
fn arithmetic_failure_rolls_back_all_writes_and_preserves_outputs_time_and_count() {
    let (program, fail, output, internal) = transactional_program(false);
    let mut r = runtime(program, 2_000, 100);
    r.single_scan().unwrap();
    let before = r.snapshot();
    assert_eq!(value(&before, output), PlcValue::Bool(true));
    assert_eq!(value(&before, internal), PlcValue::Bool(true));
    r.set_input(fail, PlcValue::Bool(true)).unwrap();
    let fault = r.single_scan().unwrap_err();
    assert_eq!(fault_code(&fault), "PLC-RUNTIME-003");
    let after = r.snapshot();
    assert_eq!(after.committed(), before.committed());
    assert_eq!(after.outputs(), before.outputs());
    assert_eq!(after.time(), before.time());
    assert_eq!(after.completed_scans(), before.completed_scans());
    assert_eq!(after.state(), RuntimeState::Faulted);
    assert_eq!(after.input_image()[&fail], PlcValue::Bool(true));
    assert!(!after.committed().contains_key(&fail));
    assert!(fault.diagnostic().unwrap().source().is_some());
    assert!(fault.diagnostic().unwrap().related().is_some());
    assert_eq!(r.single_scan(), Err(ScanError::Faulted));
    assert_eq!(r.snapshot(), after);
}
#[test]
fn budget_exhaustion_after_successful_writes_rolls_back_everything() {
    let (program, fail, output, internal) = transactional_program(true);
    let mut r = runtime(program, 100, 6);
    assert_eq!(r.single_scan().unwrap().operations(), 6);
    let before = r.snapshot();
    r.set_input(fail, PlcValue::Bool(true)).unwrap();
    let error = r.single_scan().unwrap_err();
    assert_eq!(fault_code(&error), "PLC-RUNTIME-001");
    assert_eq!(r.snapshot().committed(), before.committed());
    assert_eq!(r.snapshot().outputs(), before.outputs());
    assert_eq!(r.snapshot().time(), before.time());
    assert_eq!(r.snapshot().completed_scans(), 1);
    assert_eq!(value(&r.snapshot(), output), PlcValue::Bool(true));
    assert_eq!(value(&r.snapshot(), internal), PlcValue::Bool(true));
    assert!(error.diagnostic().unwrap().source().is_some());
    assert_eq!(
        error.diagnostic().unwrap().related(),
        Some(RelatedIdentity::Symbol(output))
    );
}
#[test]
fn identical_program_initial_defaults_inputs_and_period_have_identical_snapshots_and_faults() {
    let (program, fail, _, _) = transactional_program(false);
    let mut a = runtime(program.clone(), 333, 100);
    let mut b = runtime(program, 333, 100);
    assert_eq!(a.snapshot(), b.snapshot());
    for input in [false, false, true] {
        a.set_input(fail, PlcValue::Bool(input)).unwrap();
        b.set_input(fail, PlcValue::Bool(input)).unwrap();
        assert_eq!(a.single_scan(), b.single_scan());
        assert_eq!(a.snapshot(), b.snapshot());
    }
}
#[test]
fn explicit_reset_restores_all_defaults_clock_count_and_fault_deterministically() {
    let (program, fail, _, _) = transactional_program(false);
    let mut r = runtime(program.clone(), 100, 100);
    let fresh = runtime(program, 100, 100);
    r.single_scan().unwrap();
    r.set_input(fail, PlcValue::Bool(true)).unwrap();
    r.single_scan().unwrap_err();
    r.set_input(fail, PlcValue::Bool(false)).unwrap();
    assert_eq!(r.state(), RuntimeState::Faulted);
    r.reset();
    assert_eq!(r.snapshot(), fresh.snapshot());
    assert_eq!(r.single_scan().unwrap().snapshot().completed_scans(), 1);
}

#[test]
fn checked_int_add_subtract_multiply_negation_and_division_fault() {
    for (op, a, b, code) in [
        (B::Add, i16::MAX, 1, "PLC-RUNTIME-002"),
        (B::Subtract, i16::MIN, 1, "PLC-RUNTIME-002"),
        (B::Multiply, i16::MAX, 2, "PLC-RUNTIME-002"),
        (B::Divide, 1, 0, "PLC-RUNTIME-003"),
        (B::Divide, i16::MIN, -1, "PLC-RUNTIME-002"),
    ] {
        let (mut r, target) = binary_runtime(op, PlcValue::Int(a), PlcValue::Int(b), PlcType::Int);
        let error = r.single_scan().unwrap_err();
        assert_eq!(fault_code(&error), code);
        assert_eq!(value(&r.snapshot(), target), PlcValue::Int(0));
        assert_eq!(r.snapshot().time(), PlcTime::ZERO);
        assert_eq!(r.snapshot().completed_scans(), 0);
        assert_eq!(
            error.diagnostic().unwrap().related(),
            Some(RelatedIdentity::Symbol(target))
        );
    }
    let (mut r, _) = unary_runtime(U::Negate, PlcValue::Int(i16::MIN));
    assert_eq!(fault_code(&r.single_scan().unwrap_err()), "PLC-RUNTIME-002");
}
#[test]
fn valid_int_arithmetic_uses_signed_truncating_division_and_boundary_values() {
    for (op, a, b, expected) in [
        (B::Add, i16::MAX, 0, i16::MAX),
        (B::Subtract, i16::MIN, 0, i16::MIN),
        (B::Multiply, -7, 2, -14),
        (B::Divide, -7, 2, -3),
        (B::Divide, 7, -2, -3),
        (B::Divide, -7, -2, 3),
    ] {
        let (mut r, target) = binary_runtime(op, PlcValue::Int(a), PlcValue::Int(b), PlcType::Int);
        assert_eq!(
            value(r.single_scan().unwrap().snapshot(), target),
            PlcValue::Int(expected)
        );
    }
    let (mut r, target) = unary_runtime(U::Negate, PlcValue::Int(-1));
    assert_eq!(
        value(r.single_scan().unwrap().snapshot(), target),
        PlcValue::Int(1)
    );
}
#[test]
fn real_nonfinite_arithmetic_cannot_enter_authoritative_memory() {
    for (op, a, b) in [
        (B::Add, f32::MAX, f32::MAX),
        (B::Subtract, f32::MIN, f32::MAX),
        (B::Multiply, f32::MAX, 2.0),
        (B::Divide, 1.0, 0.0),
        (B::Divide, 0.0, 0.0),
    ] {
        let (mut r, target) = binary_runtime(
            op,
            PlcValue::real(a).unwrap(),
            PlcValue::real(b).unwrap(),
            PlcType::Real,
        );
        assert_eq!(fault_code(&r.single_scan().unwrap_err()), "PLC-RUNTIME-004");
        let PlcValue::Real(v) = value(&r.snapshot(), target) else {
            panic!("REAL expected")
        };
        assert_eq!(v.get().to_bits(), 0);
    }
}
#[test]
fn real_results_have_fixed_binary32_bits_subnormals_and_canonical_zero() {
    for (op, a, b, bits) in [
        (B::Add, 0.1, 0.2, 0x3e99_999a),
        (B::Subtract, 3.5, 1.25, 0x4010_0000),
        (B::Multiply, f32::from_bits(1), 2.0, 2),
        (B::Divide, 1.0, 3.0, 0x3eaa_aaab),
        (B::Divide, f32::from_bits(1), 2.0, 0),
        (B::Multiply, 0.0, -1.0, 0),
    ] {
        let (mut r, target) = binary_runtime(
            op,
            PlcValue::real(a).unwrap(),
            PlcValue::real(b).unwrap(),
            PlcType::Real,
        );
        let PlcValue::Real(v) = value(r.single_scan().unwrap().snapshot(), target) else {
            panic!("REAL expected")
        };
        assert_eq!(v.get().to_bits(), bits);
    }
    let (mut r, target) = unary_runtime(U::Negate, PlcValue::real(0.0).unwrap());
    let PlcValue::Real(v) = value(r.single_scan().unwrap().snapshot(), target) else {
        panic!("REAL expected")
    };
    assert_eq!(v.get().to_bits(), 0);
}
#[test]
fn real_operation_nesting_is_not_reassociated_or_constant_folded() {
    for (left_group, expected) in [(true, 0), (false, 0x3f80_0000)] {
        let mut f = Fixture::new();
        let target = f.declare("Result", PlcType::Real, Output);
        let a = f.constant(PlcValue::real(16_777_216.0).unwrap());
        let b = f.constant(PlcValue::real(1.0).unwrap());
        let c = f.constant(PlcValue::real(-16_777_216.0).unwrap());
        let expression = if left_group {
            let ab = f.binary(B::Add, PlcType::Real, a, b);
            f.binary(B::Add, PlcType::Real, ab, c)
        } else {
            let bc = f.binary(B::Add, PlcType::Real, b, c);
            f.binary(B::Add, PlcType::Real, a, bc)
        };
        let assignment = f.assign(target, expression);
        let mut r = runtime(f.finish(vec![assignment]), 1, 6);
        let scan = r.single_scan().unwrap();
        let PlcValue::Real(v) = value(scan.snapshot(), target) else {
            panic!("REAL expected")
        };
        assert_eq!(v.get().to_bits(), expected);
        assert_eq!(scan.operations(), 6);
    }
}

#[test]
fn time_comparisons_use_exact_u64_duration_ordering() {
    for (op, expected) in [
        (B::Equal, false),
        (B::NotEqual, true),
        (B::Less, false),
        (B::LessEqual, false),
        (B::Greater, true),
        (B::GreaterEqual, true),
    ] {
        let (mut r, target) = binary_runtime(
            op,
            PlcValue::Time(PlcTime::from_micros(u64::MAX)),
            PlcValue::Time(PlcTime::ZERO),
            PlcType::Bool,
        );
        assert_eq!(
            value(r.single_scan().unwrap().snapshot(), target),
            PlcValue::Bool(expected)
        );
    }
}
#[test]
fn bool_numeric_comparisons_and_not_or_follow_alpha_rules() {
    for (a, b, equals) in [
        (PlcValue::Bool(true), PlcValue::Bool(false), false),
        (PlcValue::Int(-1), PlcValue::Int(-1), true),
        (
            PlcValue::real(1.25).unwrap(),
            PlcValue::real(1.25).unwrap(),
            true,
        ),
    ] {
        for (op, expected) in [(B::Equal, equals), (B::NotEqual, !equals)] {
            let (mut r, target) = binary_runtime(op, a, b, PlcType::Bool);
            assert_eq!(
                value(r.single_scan().unwrap().snapshot(), target),
                PlcValue::Bool(expected)
            );
        }
    }
    for op in [B::Less, B::LessEqual, B::Greater, B::GreaterEqual] {
        for (a, b) in [
            (PlcValue::Int(-1), PlcValue::Int(1)),
            (PlcValue::real(-1.0).unwrap(), PlcValue::real(1.0).unwrap()),
        ] {
            let expected = matches!(op, B::Less | B::LessEqual);
            let (mut r, target) = binary_runtime(op, a, b, PlcType::Bool);
            assert_eq!(
                value(r.single_scan().unwrap().snapshot(), target),
                PlcValue::Bool(expected)
            );
        }
    }
    for (a, b, expected) in [
        (false, false, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ] {
        let (mut r, target) =
            binary_runtime(B::Or, PlcValue::Bool(a), PlcValue::Bool(b), PlcType::Bool);
        assert_eq!(
            value(r.single_scan().unwrap().snapshot(), target),
            PlcValue::Bool(expected)
        );
    }
    let (mut r, target) = unary_runtime(U::Not, PlcValue::Bool(false));
    assert_eq!(
        value(r.single_scan().unwrap().snapshot(), target),
        PlcValue::Bool(true)
    );
}

#[test]
fn unselected_if_faulting_arithmetic_is_never_executed() {
    let mut f = Fixture::new();
    let select = f.declare("Select", PlcType::Bool, Input);
    let target = f.declare("Result", PlcType::Int, Output);
    let a = f.constant(PlcValue::Int(1));
    let b = f.constant(PlcValue::Int(0));
    let division = f.binary(B::Divide, PlcType::Int, a, b);
    let bad = f.assign(target, division);
    let seven = f.constant(PlcValue::Int(7));
    let good = f.assign(target, seven);
    let condition = f.read(select);
    let branch = f.conditional(condition, vec![bad], vec![good]);
    let mut r = runtime(f.finish(vec![branch]), 1, 100);
    assert_eq!(
        value(r.single_scan().unwrap().snapshot(), target),
        PlcValue::Int(7)
    );
    r.set_input(select, PlcValue::Bool(true)).unwrap();
    assert_eq!(fault_code(&r.single_scan().unwrap_err()), "PLC-RUNTIME-003");
    assert_eq!(value(&r.snapshot(), target), PlcValue::Int(7));
}
#[test]
fn eager_and_or_evaluate_the_right_operand_even_when_left_determines_the_boolean() {
    for (op, left_value) in [(B::And, false), (B::Or, true)] {
        let mut f = Fixture::new();
        let target = f.declare("Result", PlcType::Bool, Output);
        let one = f.constant(PlcValue::Int(1));
        let zero = f.constant(PlcValue::Int(0));
        let division = f.binary(B::Divide, PlcType::Int, one, zero);
        let limit = f.constant(PlcValue::Int(0));
        let right = f.binary(B::Greater, PlcType::Bool, division, limit);
        let left = f.constant(PlcValue::Bool(left_value));
        let expression = f.binary(op, PlcType::Bool, left, right);
        let assignment = f.assign(target, expression);
        let mut r = runtime(f.finish(vec![assignment]), 1, 100);
        assert_eq!(fault_code(&r.single_scan().unwrap_err()), "PLC-RUNTIME-003");
    }
}
#[test]
fn operand_evaluation_order_is_left_to_right_with_the_first_fault_preserved() {
    for (overflow_first, expected) in [(true, "PLC-RUNTIME-002"), (false, "PLC-RUNTIME-003")] {
        let mut f = Fixture::new();
        let target = f.declare("Result", PlcType::Int, Output);
        let max = f.constant(PlcValue::Int(i16::MAX));
        let one = f.constant(PlcValue::Int(1));
        let overflow = f.binary(B::Add, PlcType::Int, max, one);
        let one = f.constant(PlcValue::Int(1));
        let zero = f.constant(PlcValue::Int(0));
        let division = f.binary(B::Divide, PlcType::Int, one, zero);
        let expected_source = if overflow_first {
            overflow.source()
        } else {
            division.source()
        };
        let expression = if overflow_first {
            f.binary(B::Add, PlcType::Int, overflow, division)
        } else {
            f.binary(B::Add, PlcType::Int, division, overflow)
        };
        let assignment = f.assign(target, expression);
        let mut r = runtime(f.finish(vec![assignment]), 1, 100);
        let error = r.single_scan().unwrap_err();
        assert_eq!(fault_code(&error), expected);
        assert_eq!(
            error.diagnostic().unwrap().source().unwrap().reference(),
            expected_source
        );
    }
}

#[test]
fn exact_budget_boundary_and_empty_program_are_unambiguous() {
    let mut f = Fixture::new();
    let target = f.declare("Result", PlcType::Bool, Output);
    let v = f.constant(PlcValue::Bool(true));
    let assignment = f.assign(target, v);
    let program = f.finish(vec![assignment]);
    let mut success = runtime(program.clone(), 1, 2);
    assert_eq!(success.single_scan().unwrap().operations(), 2);
    let mut fail = runtime(program, 1, 1);
    assert_eq!(
        fault_code(&fail.single_scan().unwrap_err()),
        "PLC-RUNTIME-001"
    );
    assert_eq!(value(&fail.snapshot(), target), PlcValue::Bool(false));
    let mut empty = runtime(Fixture::new().finish(vec![]), 1, 1);
    assert_eq!(empty.single_scan().unwrap().operations(), 0);
    assert_eq!(empty.snapshot().completed_scans(), 1);
}
#[test]
fn invalid_runtime_configuration_is_rejected_and_maximums_are_explicit() {
    assert_eq!(
        RuntimeConfig::new(PlcTime::ZERO, 1),
        Err(ConfigurationError::ZeroCyclePeriod)
    );
    assert_eq!(
        RuntimeConfig::new(PlcTime::from_micros(1), 0),
        Err(ConfigurationError::ZeroExecutionBudget)
    );
    let c = config(u64::MAX, u64::MAX);
    assert_eq!(c.cycle_period().as_micros(), u64::MAX);
    assert_eq!(c.execution_budget(), u64::MAX);
}
#[test]
fn exact_cycles_advance_once_and_clock_overflow_rolls_back_completion() {
    let mut f = Fixture::new();
    let target = f.declare("Count", PlcType::Int, Output);
    let old = f.read(target);
    let one = f.constant(PlcValue::Int(1));
    let next = f.binary(B::Add, PlcType::Int, old, one);
    let assignment = f.assign(target, next);
    let program = f.finish(vec![assignment]);
    let mut regular = runtime(program.clone(), 777, 4);
    for n in 1..=10 {
        let scan = regular.single_scan().unwrap();
        assert_eq!(scan.snapshot().time().as_micros(), n * 777);
        assert_eq!(scan.snapshot().completed_scans(), n);
    }
    let mut limit = runtime(program, u64::MAX, 4);
    limit.single_scan().unwrap();
    let before = limit.snapshot();
    let fault = limit.single_scan().unwrap_err();
    assert_eq!(fault_code(&fault), "PLC-RUNTIME-005");
    assert!(fault.diagnostic().unwrap().source().is_none());
    assert_eq!(limit.snapshot().time(), before.time());
    assert_eq!(limit.snapshot().completed_scans(), 1);
    assert_eq!(limit.snapshot().committed(), before.committed());
    assert_eq!(value(&limit.snapshot(), target), PlcValue::Int(1));
}

#[test]
fn input_api_checks_id_role_type_and_never_mutates_on_rejection() {
    let mut f = Fixture::new();
    let input = f.declare("Input", PlcType::Int, Input);
    let output = f.declare("Output", PlcType::Int, Output);
    let program = f.finish(vec![]);
    let mut r = runtime(program, 1, 1);
    let before = r.snapshot();
    assert_eq!(
        r.set_input(sid(99), PlcValue::Int(1)),
        Err(InputError::UnknownSymbol(sid(99)))
    );
    assert_eq!(
        r.set_input(output, PlcValue::Int(1)),
        Err(InputError::NotExternalInput(output))
    );
    assert_eq!(
        r.set_input(input, PlcValue::Bool(true)),
        Err(InputError::TypeMismatch {
            symbol: input,
            expected: PlcType::Int,
            actual: PlcType::Bool
        })
    );
    assert_eq!(r.snapshot(), before);
    r.set_input(input, PlcValue::Int(i16::MAX)).unwrap();
    assert_eq!(r.state(), RuntimeState::Ready);
}
#[test]
fn compiled_roles_apply_only_to_the_entry_and_globals_not_other_pous() {
    let mut f = Fixture::new();
    let input = f.declare("Input", PlcType::Bool, Input);
    let output = f.declare("Output", PlcType::Bool, Output);
    let local = f.declare("Local", PlcType::Bool, Local);
    let global = sid(99);
    let global_value = f.constant(PlcValue::Bool(true));
    let write = f.assign(global, global_value);
    let base = f.finish(vec![write]);
    let mut symbols = base.symbols().clone();
    let foreign = PouId::new(2).unwrap();
    symbols
        .declare_pou(Pou::new(foreign, "Foreign", PouKind::FunctionBlock).unwrap())
        .unwrap();
    for (id, name, category) in [
        (10, "ForeignInput", Input),
        (11, "ForeignOutput", Output),
        (12, "ForeignInOut", InOut),
    ] {
        symbols
            .declare(
                Symbol::new(
                    sid(id),
                    name,
                    PlcType::Bool,
                    Scope::Pou(foreign),
                    category,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
    }
    symbols
        .declare(Symbol::new(global, "Global", PlcType::Bool, Scope::Global, Local, None).unwrap())
        .unwrap();
    let program = Program::new(
        symbols,
        base.sources().clone(),
        entry(),
        base.body().to_vec(),
    );
    let plan = lower(&validate(&program).unwrap()).unwrap();
    for (id, role) in [
        (input, MemoryRole::ExternalInput),
        (output, MemoryRole::ExternalOutput),
        (local, MemoryRole::Internal),
        (global, MemoryRole::Internal),
    ] {
        assert_eq!(plan.memory_role(id), Some(role));
    }
    assert_eq!(plan.memory_role(sid(10)), None);
    assert_eq!(plan.memory_role(sid(11)), None);
    assert_eq!(plan.memory_role(sid(12)), None);
    let mut r = Runtime::new(plan, config(1, 2));
    assert!(r.set_input(sid(10), PlcValue::Bool(true)).is_err());
    assert_eq!(
        value(r.single_scan().unwrap().snapshot(), global),
        PlcValue::Bool(true)
    );
    assert!(!r.snapshot().outputs().contains_key(&sid(11)));
}
#[test]
fn unsupported_entry_inout_declaration_fails_checked_lowering() {
    let mut f = Fixture::new();
    let id = f.declare("Parameter", PlcType::Int, InOut);
    let program = f.finish(vec![]);
    let checked = validate(&program).unwrap();
    let error = lower(&checked).unwrap_err();
    assert!(matches!(error, LoweringError::Unsupported(_)));
    assert_eq!(error.diagnostic().code(), "PLC-COMP-015");
    assert_eq!(
        error.diagnostic().category(),
        DiagnosticCategory::CompilerDiagnostic
    );
    assert_eq!(
        error.diagnostic().related(),
        Some(RelatedIdentity::Symbol(id))
    );
    assert!(error.diagnostic().source().is_some());
}
#[test]
fn all_memory_roles_use_authoritative_type_defaults() {
    let mut f = Fixture::new();
    let mut expected = vec![];
    for (index, t) in [PlcType::Bool, PlcType::Int, PlcType::Real, PlcType::Time]
        .into_iter()
        .enumerate()
    {
        for category in [Input, Output, Local] {
            let name = format!("V{index}{category:?}");
            let id = f.declare(&name, t, category);
            expected.push((id, t.default_value(), category));
        }
    }
    let r = runtime(f.finish(vec![]), 1, 1);
    let snapshot = r.snapshot();
    for (id, expected, category) in expected {
        if category == Input {
            assert_eq!(snapshot.staged_inputs()[&id], expected);
            assert_eq!(snapshot.input_image()[&id], expected);
            assert!(!snapshot.committed().contains_key(&id));
        } else {
            assert_eq!(value(&snapshot, id), expected);
            if category == Output {
                assert_eq!(snapshot.outputs()[&id], expected);
            }
        }
        if let PlcValue::Real(real) = expected {
            assert_eq!(real.get().to_bits(), 0);
        }
    }
}

#[test]
fn entry_locals_and_globals_persist_across_successful_scans_with_a_fresh_budget() {
    let mut f = Fixture::new();
    let local = f.declare("LocalCount", PlcType::Int, Local);
    let output = f.declare("Total", PlcType::Int, Output);
    let global = sid(99);
    let old_local = f.read(local);
    let one = f.constant(PlcValue::Int(1));
    let next_local = f.binary(B::Add, PlcType::Int, old_local, one);
    let local_write = f.assign(local, next_local);
    let old_global = Expression::read(f.anchor(), global, PlcType::Int);
    let two = f.constant(PlcValue::Int(2));
    let next_global = f.binary(B::Add, PlcType::Int, old_global, two);
    let global_write = f.assign(global, next_global);
    let local_value = f.read(local);
    let global_value = Expression::read(f.anchor(), global, PlcType::Int);
    let total = f.binary(B::Add, PlcType::Int, local_value, global_value);
    let output_write = f.assign(output, total);
    let base = f.finish(vec![local_write, global_write, output_write]);
    let mut symbols = base.symbols().clone();
    symbols
        .declare(
            Symbol::new(
                global,
                "GlobalCount",
                PlcType::Int,
                Scope::Global,
                Local,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    let program = Program::new(
        symbols,
        base.sources().clone(),
        entry(),
        base.body().to_vec(),
    );
    let mut r = runtime(program, 1, 12);
    for n in 1..=3 {
        let scan = r.single_scan().unwrap();
        assert_eq!(scan.operations(), 12);
        assert_eq!(value(scan.snapshot(), local), PlcValue::Int(n));
        assert_eq!(value(scan.snapshot(), global), PlcValue::Int(2 * n));
        assert_eq!(value(scan.snapshot(), output), PlcValue::Int(3 * n));
    }
}
