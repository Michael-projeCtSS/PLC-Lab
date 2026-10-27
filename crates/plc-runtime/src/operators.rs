//! Exact Alpha operations. No optimization, promotion, TIME arithmetic, or host capabilities.
use crate::errors::{FaultKind, InternalError};
use plc_types::{
    PlcReal, PlcValue,
    compatibility::{self, BinaryOperator, ConversionFunction, UnaryOperator},
};
use std::cmp::Ordering;

pub(crate) enum OperationError {
    Fault(FaultKind),
    Internal(InternalError),
}
fn invalid_operands() -> OperationError {
    OperationError::Internal(InternalError::new(
        "Validated execution operands have incompatible types.",
    ))
}
fn int(result: Option<i16>) -> Result<PlcValue, OperationError> {
    result
        .map(PlcValue::Int)
        .ok_or(OperationError::Fault(FaultKind::IntOverflow))
}
fn real(result: f32) -> Result<PlcValue, OperationError> {
    PlcReal::new(result)
        .map(PlcValue::Real)
        .map_err(|_| OperationError::Fault(FaultKind::NonFiniteReal))
}

pub(crate) fn unary(op: UnaryOperator, operand: PlcValue) -> Result<PlcValue, OperationError> {
    match (op, operand) {
        (UnaryOperator::Not, PlcValue::Bool(v)) => Ok(PlcValue::Bool(!v)),
        (UnaryOperator::Negate, PlcValue::Int(v)) => int(v.checked_neg()),
        (UnaryOperator::Negate, PlcValue::Real(v)) => real(-v.get()),
        _ => Err(invalid_operands()),
    }
}
pub(crate) fn binary(
    op: BinaryOperator,
    left: PlcValue,
    right: PlcValue,
) -> Result<PlcValue, OperationError> {
    if compatibility::binary_result(op, left.plc_type(), right.plc_type()).is_none() {
        return Err(invalid_operands());
    }
    use BinaryOperator::*;
    match op {
        Equal => return Ok(PlcValue::Bool(left == right)),
        NotEqual => return Ok(PlcValue::Bool(left != right)),
        Less | LessEqual | Greater | GreaterEqual => {
            let order = match (left, right) {
                (PlcValue::Int(a), PlcValue::Int(b)) => Some(a.cmp(&b)),
                (PlcValue::Real(a), PlcValue::Real(b)) => a.get().partial_cmp(&b.get()),
                (PlcValue::Time(a), PlcValue::Time(b)) => Some(a.cmp(&b)),
                _ => None,
            }
            .ok_or_else(invalid_operands)?;
            return Ok(PlcValue::Bool(match op {
                Less => order == Ordering::Less,
                LessEqual => order != Ordering::Greater,
                Greater => order == Ordering::Greater,
                GreaterEqual => order != Ordering::Less,
                _ => return Err(invalid_operands()),
            }));
        }
        _ => {}
    }
    match (op, left, right) {
        (And, PlcValue::Bool(a), PlcValue::Bool(b)) => Ok(PlcValue::Bool(a && b)),
        (Or, PlcValue::Bool(a), PlcValue::Bool(b)) => Ok(PlcValue::Bool(a || b)),
        (Xor, PlcValue::Bool(a), PlcValue::Bool(b)) => Ok(PlcValue::Bool(a ^ b)),
        (Modulo, PlcValue::Int(_), PlcValue::Int(0)) => {
            Err(OperationError::Fault(FaultKind::DivisionByZero))
        }
        // i16::MIN MOD -1 is mathematically 0; checked_rem reports it as overflow.
        (Modulo, PlcValue::Int(_), PlcValue::Int(-1)) => Ok(PlcValue::Int(0)),
        (Modulo, PlcValue::Int(a), PlcValue::Int(b)) => int(a.checked_rem(b)),
        (Add, PlcValue::Int(a), PlcValue::Int(b)) => int(a.checked_add(b)),
        (Subtract, PlcValue::Int(a), PlcValue::Int(b)) => int(a.checked_sub(b)),
        (Multiply, PlcValue::Int(a), PlcValue::Int(b)) => int(a.checked_mul(b)),
        (Divide, PlcValue::Int(_), PlcValue::Int(0)) => {
            Err(OperationError::Fault(FaultKind::DivisionByZero))
        }
        (Divide, PlcValue::Int(a), PlcValue::Int(b)) => int(a.checked_div(b)),
        (Add, PlcValue::Real(a), PlcValue::Real(b)) => real(a.get() + b.get()),
        (Subtract, PlcValue::Real(a), PlcValue::Real(b)) => real(a.get() - b.get()),
        (Multiply, PlcValue::Real(a), PlcValue::Real(b)) => real(a.get() * b.get()),
        (Divide, PlcValue::Real(a), PlcValue::Real(b)) => real(a.get() / b.get()),
        _ => Err(invalid_operands()),
    }
}

/// Eager associative BOOL operation over already evaluated operands.
pub(crate) fn logical(op: BinaryOperator, values: &[PlcValue]) -> Result<PlcValue, OperationError> {
    let mut bits = Vec::with_capacity(values.len());
    for v in values {
        let PlcValue::Bool(b) = v else {
            return Err(invalid_operands());
        };
        bits.push(*b);
    }
    match op {
        BinaryOperator::And => Ok(PlcValue::Bool(bits.iter().all(|b| *b))),
        BinaryOperator::Or => Ok(PlcValue::Bool(bits.iter().any(|b| *b))),
        BinaryOperator::Xor => Ok(PlcValue::Bool(bits.iter().fold(false, |a, b| a ^ b))),
        _ => Err(invalid_operands()),
    }
}

pub(crate) fn convert(
    function: ConversionFunction,
    operand: PlcValue,
) -> Result<PlcValue, OperationError> {
    let to_int = |v: f32| {
        if (f32::from(i16::MIN)..=f32::from(i16::MAX)).contains(&v) {
            Ok(PlcValue::Int(v as i16))
        } else {
            Err(OperationError::Fault(FaultKind::IntOverflow))
        }
    };
    match (function, operand) {
        (ConversionFunction::IntToReal, PlcValue::Int(i)) => real(f32::from(i)),
        (ConversionFunction::RealToInt, PlcValue::Real(r)) => to_int(r.get().round_ties_even()),
        (ConversionFunction::Trunc, PlcValue::Real(r)) => to_int(r.get().trunc()),
        _ => Err(invalid_operands()),
    }
}
