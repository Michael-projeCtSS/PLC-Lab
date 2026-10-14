mod support;
use plc_ir::*;
use plc_types::{
    PlcTime, PlcType, PlcValue,
    compatibility::{BinaryOperator, UnaryOperator},
    diagnostics::{DiagnosticCategory, RelatedIdentity, Severity},
    identity::{FunctionBlockInstanceId, ParameterId, PouId},
    source::SourceMap,
    symbols::{
        Pou, PouKind, Scope, Symbol, SymbolTable,
        VariableCategory::{Input, Local, Output},
    },
};
use support::*;

fn codes(program: &Program) -> Vec<String> {
    validate(program)
        .unwrap_err()
        .iter()
        .map(|d| d.code().into())
        .collect()
}

#[test]
fn entry_input_assignment_is_rejected_with_source_and_identity() {
    let mut f = Fixture::new();
    let input = f.declare("Input", PlcType::Bool, Input);
    let value = f.constant(PlcValue::Bool(true));
    let assignment = f.assign(input, value);
    let program = f.finish(vec![assignment]);
    let errors = validate(&program).unwrap_err();
    assert_eq!(codes(&program), ["PLC-COMP-014"]);
    assert_eq!(errors[0].related(), Some(RelatedIdentity::Symbol(input)));
    assert!(errors[0].source().is_some());
}

#[test]
fn output_local_and_global_assignments_remain_semantically_valid() {
    let mut f = Fixture::new();
    let output = f.declare("Output", PlcType::Bool, Output);
    let local = f.declare("Local", PlcType::Bool, Local);
    let value = f.constant(PlcValue::Bool(true));
    let output_write = f.assign(output, value.clone());
    let local_write = f.assign(local, value.clone());
    let global = sid(99);
    let global_write = f.assign(global, value);
    let base = f.finish(vec![output_write, local_write, global_write]);
    let mut symbols = base.symbols().clone();
    symbols
        .declare(Symbol::new(global, "Global", PlcType::Bool, Scope::Global, Local, None).unwrap())
        .unwrap();
    validate(&Program::new(
        symbols,
        base.sources().clone(),
        entry(),
        base.body().to_vec(),
    ))
    .unwrap();
}

#[test]
fn unknown_read_and_write_return_student_diagnostics_with_source_and_symbol() {
    let mut f = Fixture::new();
    let value = Expression::read(f.anchor(), sid(99), PlcType::Bool);
    let assignment = f.assign(sid(98), value);
    let program = f.finish(vec![assignment]);
    let errors = validate(&program).unwrap_err();
    assert_eq!(codes(&program), ["PLC-COMP-001", "PLC-COMP-001"]);
    assert_eq!(errors[0].related(), Some(RelatedIdentity::Symbol(sid(98))));
    assert_eq!(errors[1].related(), Some(RelatedIdentity::Symbol(sid(99))));
    for error in errors {
        assert!(error.source().is_some());
        assert_eq!(error.category(), DiagnosticCategory::CompilerDiagnostic);
        assert_eq!(error.severity(), Severity::Error);
    }
}
#[test]
fn assignment_rejects_numeric_conversion_in_both_directions() {
    for (target_type, value) in [
        (PlcType::Real, PlcValue::Int(1)),
        (PlcType::Int, PlcValue::real(1.0).unwrap()),
        (PlcType::Bool, PlcValue::Int(1)),
    ] {
        let mut f = Fixture::new();
        let target = f.declare("Target", target_type, Output);
        let value = f.constant(value);
        let assignment = f.assign(target, value);
        assert_eq!(codes(&f.finish(vec![assignment])), ["PLC-COMP-007"]);
    }
}
#[test]
fn read_and_operator_result_type_claims_are_checked() {
    let mut f = Fixture::new();
    let start = f.declare("Start", PlcType::Bool, Input);
    let target = f.declare("Target", PlcType::Int, Output);
    let value = Expression::read(f.anchor(), start, PlcType::Int);
    let statement = f.assign(target, value);
    assert_eq!(codes(&f.finish(vec![statement])), ["PLC-COMP-008"]);
    let mut f = Fixture::new();
    let target = f.declare("Target", PlcType::Int, Output);
    let yes = f.constant(PlcValue::Bool(true));
    let value = Expression::unary(f.anchor(), PlcType::Int, UnaryOperator::Not, yes);
    let assignment = f.assign(target, value);
    assert_eq!(codes(&f.finish(vec![assignment])), ["PLC-COMP-008"]);
    let mut f = Fixture::new();
    let target = f.declare("Target", PlcType::Int, Output);
    let left = f.constant(PlcValue::Int(1));
    let right = f.constant(PlcValue::Int(2));
    let value = f.binary(BinaryOperator::Greater, PlcType::Int, left, right);
    let assignment = f.assign(target, value);
    assert_eq!(codes(&f.finish(vec![assignment])), ["PLC-COMP-008"]);
}
#[test]
fn invalid_boolean_arithmetic_and_comparison_operands_are_rejected() {
    for (operator, t, a, b) in [
        (
            BinaryOperator::And,
            PlcType::Bool,
            PlcValue::Int(1),
            PlcValue::Int(0),
        ),
        (
            BinaryOperator::Or,
            PlcType::Bool,
            PlcValue::Bool(true),
            PlcValue::Int(0),
        ),
        (
            BinaryOperator::Add,
            PlcType::Time,
            PlcValue::Time(PlcTime::ZERO),
            PlcValue::Time(PlcTime::from_micros(1)),
        ),
        (
            BinaryOperator::Greater,
            PlcType::Bool,
            PlcValue::Bool(true),
            PlcValue::Bool(false),
        ),
        (
            BinaryOperator::Equal,
            PlcType::Bool,
            PlcValue::Int(1),
            PlcValue::real(1.0).unwrap(),
        ),
    ] {
        let mut f = Fixture::new();
        let target = f.declare("Target", t, Output);
        let left = f.constant(a);
        let right = f.constant(b);
        let value = f.binary(operator, t, left, right);
        let statement = f.assign(target, value);
        assert_eq!(codes(&f.finish(vec![statement])), ["PLC-COMP-008"]);
    }
    for (operator, operand) in [
        (UnaryOperator::Not, PlcValue::Int(1)),
        (UnaryOperator::Negate, PlcValue::Bool(true)),
    ] {
        let mut f = Fixture::new();
        let target = f.declare("Target", PlcType::Bool, Output);
        let operand = f.constant(operand);
        let value = Expression::unary(f.anchor(), PlcType::Bool, operator, operand);
        let assignment = f.assign(target, value);
        assert_eq!(codes(&f.finish(vec![assignment])), ["PLC-COMP-008"]);
    }
}
#[test]
fn numeric_and_time_comparisons_and_boolean_or_validate_without_evaluation() {
    for (operator, a, b, t) in [
        (
            BinaryOperator::Add,
            PlcValue::Int(i16::MAX),
            PlcValue::Int(1),
            PlcType::Int,
        ),
        (
            BinaryOperator::Divide,
            PlcValue::Int(1),
            PlcValue::Int(0),
            PlcType::Int,
        ),
        (
            BinaryOperator::Subtract,
            PlcValue::real(2.0).unwrap(),
            PlcValue::real(1.0).unwrap(),
            PlcType::Real,
        ),
        (
            BinaryOperator::Multiply,
            PlcValue::real(2.0).unwrap(),
            PlcValue::real(3.0).unwrap(),
            PlcType::Real,
        ),
        (
            BinaryOperator::LessEqual,
            PlcValue::Time(PlcTime::ZERO),
            PlcValue::Time(PlcTime::from_micros(1)),
            PlcType::Bool,
        ),
        (
            BinaryOperator::Or,
            PlcValue::Bool(true),
            PlcValue::Bool(false),
            PlcType::Bool,
        ),
    ] {
        let mut f = Fixture::new();
        let target = f.declare("Target", t, Output);
        let left = f.constant(a);
        let right = f.constant(b);
        let value = f.binary(operator, t, left, right);
        let statement = f.assign(target, value);
        validate(&f.finish(vec![statement])).unwrap();
    }
    let mut f = Fixture::new();
    let target = f.declare("Target", PlcType::Int, Output);
    let operand = f.constant(PlcValue::Int(i16::MIN));
    let value = Expression::unary(f.anchor(), PlcType::Int, UnaryOperator::Negate, operand);
    let assignment = f.assign(target, value);
    validate(&f.finish(vec![assignment])).unwrap();
}
#[test]
fn conditions_require_bool_and_both_branches_are_validated() {
    let mut f = Fixture::new();
    let condition = f.constant(PlcValue::Int(1));
    let branch = f.conditional(condition, vec![], vec![]);
    assert_eq!(codes(&f.finish(vec![branch])), ["PLC-COMP-009"]);
    let mut f = Fixture::new();
    let condition = f.constant(PlcValue::Bool(true));
    let value = f.constant(PlcValue::Bool(false));
    let first = f.assign(sid(98), value.clone());
    let second = f.assign(sid(99), value);
    let branch = f.conditional(condition, vec![first], vec![second]);
    assert_eq!(
        codes(&f.finish(vec![branch])),
        ["PLC-COMP-001", "PLC-COMP-001"]
    );
}
#[test]
fn entry_and_foreign_scope_references_are_rejected() {
    for kind in [None, Some(PouKind::Function), Some(PouKind::FunctionBlock)] {
        let mut symbols = SymbolTable::new();
        if let Some(kind) = kind {
            symbols
                .declare_pou(Pou::new(entry(), "Entry", kind).unwrap())
                .unwrap();
        }
        assert_eq!(
            codes(&Program::new(symbols, SourceMap::new(), entry(), vec![])),
            ["PLC-COMP-013"]
        );
    }
    let mut symbols = SymbolTable::new();
    symbols
        .declare_pou(Pou::new(entry(), "Main", PouKind::Program).unwrap())
        .unwrap();
    let other = PouId::new(2).unwrap();
    symbols
        .declare_pou(Pou::new(other, "Other", PouKind::Program).unwrap())
        .unwrap();
    symbols
        .declare(
            Symbol::new(
                sid(1),
                "Foreign",
                PlcType::Bool,
                Scope::Pou(other),
                Local,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    let f = Fixture::new();
    let empty = f.finish(vec![]);
    let value = Expression::read(source(1, 1), sid(1), PlcType::Bool);
    let assignment = Statement::assign(source(1, 1), sid(1), value);
    let mut sources = empty.sources().clone();
    sources
        .register_element(plc_types::source::SourceLocation::new(
            source(1, 1),
            plc_types::source::SourceLanguage::St,
            None,
        ))
        .unwrap();
    assert_eq!(
        codes(&Program::new(symbols, sources, entry(), vec![assignment])),
        ["PLC-COMP-003", "PLC-COMP-003"]
    );
}
#[test]
fn dangling_document_element_and_declaration_sources_are_rejected() {
    for bad_source in [source(99, 1), source(1, 99)] {
        let mut f = Fixture::new();
        let target = f.declare("Target", PlcType::Bool, Output);
        let value = Expression::constant(bad_source, PlcValue::Bool(true));
        let assignment = Statement::assign(bad_source, target, value);
        assert_eq!(
            codes(&f.finish(vec![assignment])),
            ["PLC-COMP-010", "PLC-COMP-010"]
        );
    }
    let mut symbols = SymbolTable::new();
    symbols
        .declare_pou(Pou::new(entry(), "Main", PouKind::Program).unwrap())
        .unwrap();
    symbols
        .declare(
            Symbol::new(
                sid(1),
                "Bad",
                PlcType::Bool,
                Scope::Global,
                Local,
                Some(source(1, 99)),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        codes(&Program::new(symbols, SourceMap::new(), entry(), vec![])),
        ["PLC-COMP-010"]
    );
}
#[test]
fn undeclared_function_block_call_and_duplicate_bindings_are_rejected() {
    let mut f = Fixture::new();
    let start = f.declare("Start", PlcType::Bool, Input);
    let q = f.declare("Q", PlcType::Bool, Output);
    let et = f.declare("ET", PlcType::Time, Output);
    let input = f.read(start);
    let pt = f.constant(PlcValue::Time(PlcTime::from_micros(5_000_000)));
    let parameter = |n| ParameterId::new(n).unwrap();
    let instance = FunctionBlockInstanceId::new(1).unwrap();
    let call = FunctionBlockCall::new(
        instance,
        vec![
            CallBinding::input(parameter(1), input.clone()),
            CallBinding::input(parameter(2), pt),
            CallBinding::output(parameter(3), q),
            CallBinding::output(parameter(4), et),
        ],
    )
    .unwrap();
    assert_eq!(call.instance(), instance);
    assert_eq!(call.bindings().len(), 4);
    let statement = Statement::function_block_call(f.anchor(), call);
    assert_eq!(codes(&f.finish(vec![statement])), ["PLC-COMP-016"]);
    assert_eq!(
        FunctionBlockCall::new(
            instance,
            vec![
                CallBinding::input(parameter(1), input.clone()),
                CallBinding::input(parameter(1), input.clone())
            ]
        ),
        Err(DuplicateCallParameter)
    );
    assert_eq!(
        FunctionBlockCall::new(
            instance,
            vec![
                CallBinding::input(parameter(1), input),
                CallBinding::output(parameter(1), q)
            ]
        ),
        Err(DuplicateCallParameter)
    );
}
#[test]
fn validation_is_repeatable_and_preserves_the_candidate() {
    let mut f = Fixture::new();
    let target = f.declare("Target", PlcType::Bool, Output);
    let value = f.constant(PlcValue::Int(1));
    let statement = f.assign(target, value);
    let program = f.finish(vec![statement]);
    let before = program.clone();
    assert_eq!(
        validate(&program).unwrap_err(),
        validate(&program).unwrap_err()
    );
    assert_eq!(program, before);
}
#[test]
fn validation_nesting_limit_returns_diagnostics_at_the_boundary() {
    for (depth, valid) in [
        (MAX_VALIDATION_DEPTH, true),
        (MAX_VALIDATION_DEPTH + 1, false),
    ] {
        let mut f = Fixture::new();
        let target = f.declare("Target", PlcType::Bool, Output);
        let mut value = f.constant(PlcValue::Bool(true));
        for _ in 1..depth {
            value = f.not(value);
        }
        let statement = f.assign(target, value);
        let program = f.finish(vec![statement]);
        if valid {
            validate(&program).unwrap();
        } else {
            assert_eq!(codes(&program), ["PLC-COMP-012"]);
        }
    }
}
#[test]
fn validation_node_limit_returns_diagnostics_without_unbounded_collection() {
    let mut f = Fixture::new();
    let target = f.declare("Target", PlcType::Bool, Output);
    let value = f.constant(PlcValue::Bool(true));
    let statement = f.assign(target, value);
    let accepted = f.finish(vec![statement.clone(); (MAX_VALIDATION_NODES - 1) / 2]);
    validate(&accepted).unwrap();
    let rejected = Program::new(
        accepted.symbols().clone(),
        accepted.sources().clone(),
        entry(),
        vec![statement; MAX_VALIDATION_NODES / 2],
    );
    assert_eq!(codes(&rejected), ["PLC-COMP-012"]);
}
