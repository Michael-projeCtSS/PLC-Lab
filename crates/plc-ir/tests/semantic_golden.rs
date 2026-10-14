mod support;
use plc_ir::{ExpressionKind, StatementKind, validate};
use plc_types::{
    PlcType, PlcValue,
    compatibility::{BinaryOperator, UnaryOperator},
    source::SourceLanguage,
    symbols::VariableCategory::{Input, Output},
};
use support::*;

#[test]
fn motor_assign_start() {
    let mut f = Fixture::new();
    let start = f.declare("Start", PlcType::Bool, Input);
    let motor = f.declare("Motor", PlcType::Bool, Output);
    let value = f.read(start);
    let assignment = f.assign(motor, value);
    let program = f.finish(vec![assignment]);
    let checked = validate(&program).unwrap();
    let StatementKind::Assign { target, value } = checked.program().body()[0].kind() else {
        panic!("assignment expected")
    };
    assert_eq!(*target, motor);
    assert_eq!(value.kind(), &ExpressionKind::Read(start));
    assert_eq!(value.result_type(), PlcType::Bool);
}

#[test]
fn motor_assign_start_and_not_stop() {
    let mut f = Fixture::new();
    let start = f.declare("Start", PlcType::Bool, Input);
    let stop = f.declare("Stop", PlcType::Bool, Input);
    let motor = f.declare("Motor", PlcType::Bool, Output);
    let left = f.read(start);
    let stop_value = f.read(stop);
    let right = f.not(stop_value);
    let value = f.binary(BinaryOperator::And, PlcType::Bool, left, right);
    let assignment = f.assign(motor, value);
    let program = f.finish(vec![assignment]);
    validate(&program).unwrap();
    let StatementKind::Assign { target, value } = program.body()[0].kind() else {
        panic!("assignment expected")
    };
    assert_eq!(*target, motor);
    let ExpressionKind::Binary {
        operator,
        left,
        right,
    } = value.kind()
    else {
        panic!("binary expected")
    };
    assert_eq!(*operator, BinaryOperator::And);
    assert_eq!(left.kind(), &ExpressionKind::Read(start));
    let ExpressionKind::Unary { operator, operand } = right.kind() else {
        panic!("unary expected")
    };
    assert_eq!(*operator, UnaryOperator::Not);
    assert_eq!(operand.kind(), &ExpressionKind::Read(stop));
    for e in [value, left, right, operand] {
        assert_eq!(e.result_type(), PlcType::Bool);
        assert!(program.sources().get(e.source()).is_some());
    }
}

#[test]
fn if_level_high_then_pump_false_without_else() {
    let mut f = Fixture::new();
    let level = f.declare("LevelHigh", PlcType::Bool, Input);
    let pump = f.declare("Pump", PlcType::Bool, Output);
    let condition = f.read(level);
    let off = f.constant(PlcValue::Bool(false));
    let assignment = f.assign(pump, off);
    let branch = f.conditional(condition, vec![assignment], vec![]);
    let program = f.finish(vec![branch]);
    validate(&program).unwrap();
    let StatementKind::If {
        condition,
        then_body,
        else_body,
    } = program.body()[0].kind()
    else {
        panic!("conditional expected")
    };
    assert_eq!(condition.kind(), &ExpressionKind::Read(level));
    assert!(else_body.is_empty());
    assert_eq!(then_body.len(), 1);
    let StatementKind::Assign { target, value } = then_body[0].kind() else {
        panic!("assignment expected")
    };
    assert_eq!(*target, pump);
    assert_eq!(
        value.kind(),
        &ExpressionKind::Constant(PlcValue::Bool(false))
    );
}

#[test]
fn if_value_greater_limit_then_alarm_true_else_false() {
    let mut f = Fixture::new();
    let value = f.declare("Value", PlcType::Int, Input);
    let limit = f.declare("Limit", PlcType::Int, Input);
    let alarm = f.declare("Alarm", PlcType::Bool, Output);
    let left = f.read(value);
    let right = f.read(limit);
    let condition = f.binary(BinaryOperator::Greater, PlcType::Bool, left, right);
    let on = f.constant(PlcValue::Bool(true));
    let off = f.constant(PlcValue::Bool(false));
    let then_assignment = f.assign(alarm, on);
    let else_assignment = f.assign(alarm, off);
    let branch = f.conditional(condition, vec![then_assignment], vec![else_assignment]);
    let program = f.finish(vec![branch]);
    validate(&program).unwrap();
    let StatementKind::If {
        condition,
        then_body,
        else_body,
    } = program.body()[0].kind()
    else {
        panic!("conditional expected")
    };
    let ExpressionKind::Binary {
        operator,
        left,
        right,
    } = condition.kind()
    else {
        panic!("comparison expected")
    };
    assert_eq!(*operator, BinaryOperator::Greater);
    assert_eq!(left.result_type(), PlcType::Int);
    assert_eq!(right.result_type(), PlcType::Int);
    assert_eq!(condition.result_type(), PlcType::Bool);
    for (body, expected) in [(then_body, true), (else_body, false)] {
        assert_eq!(body.len(), 1);
        let StatementKind::Assign { target, value } = body[0].kind() else {
            panic!("assignment expected")
        };
        assert_eq!(*target, alarm);
        assert_eq!(
            value.kind(),
            &ExpressionKind::Constant(PlcValue::Bool(expected))
        );
    }
}

#[test]
fn identical_semantic_construction_accepts_text_and_graphical_anchors() {
    for language in [
        SourceLanguage::St,
        SourceLanguage::Lad,
        SourceLanguage::Fbd,
        SourceLanguage::Stl,
    ] {
        let mut f = Fixture::language(language);
        let start = f.declare("Start", PlcType::Bool, Input);
        let motor = f.declare("Motor", PlcType::Bool, Output);
        let value = f.read(start);
        let assignment = f.assign(motor, value);
        let program = f.finish(vec![assignment]);
        validate(&program).unwrap();
        let source = program.sources().get(program.body()[0].source()).unwrap();
        assert_eq!(source.language(), language);
        assert_eq!(
            source.text_span().is_some(),
            matches!(language, SourceLanguage::St | SourceLanguage::Stl)
        );
    }
}
