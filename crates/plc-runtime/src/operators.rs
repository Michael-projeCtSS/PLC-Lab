//! Exact Alpha operations. No optimization, promotion, TIME arithmetic, or host capabilities.
use crate::errors::{FaultKind, InternalError};
use plc_types::{
    PlcReal, PlcValue,
    compatibility::{self, BinaryOperator, UnaryOperator},
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
