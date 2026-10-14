//! Shared typing rules, without evaluating operands or inventing conversions.

use crate::PlcType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentCompatibility {
    Identity,
    /// Reserved vocabulary; no Alpha pair currently returns this classification.
    ImplicitConversion,
    /// Reserved vocabulary; no explicit conversion is approved in Alpha.
    ExplicitConversion,
    Unsupported,
}

pub fn assignment(target: PlcType, value: PlcType) -> AssignmentCompatibility {
    if target == value {
        AssignmentCompatibility::Identity
    } else {
        AssignmentCompatibility::Unsupported
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    And,
    Or,
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

pub fn unary_result(operator: UnaryOperator, operand: PlcType) -> Option<PlcType> {
    match (operator, operand) {
        (UnaryOperator::Not, PlcType::Bool) => Some(PlcType::Bool),
        (UnaryOperator::Negate, PlcType::Int | PlcType::Real) => Some(operand),
        _ => None,
    }
}

pub fn binary_result(operator: BinaryOperator, left: PlcType, right: PlcType) -> Option<PlcType> {
    if left != right {
        return None;
    }
    use BinaryOperator::*;
    match operator {
        And | Or if left == PlcType::Bool => Some(PlcType::Bool),
        Add | Subtract | Multiply | Divide if matches!(left, PlcType::Int | PlcType::Real) => {
            Some(left)
        }
        Equal | NotEqual => Some(PlcType::Bool),
        Less | LessEqual | Greater | GreaterEqual
            if matches!(left, PlcType::Int | PlcType::Real | PlcType::Time) =>
        {
            Some(PlcType::Bool)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const TYPES: [PlcType; 4] = [PlcType::Bool, PlcType::Int, PlcType::Real, PlcType::Time];

    #[test]
    fn complete_assignment_matrix_allows_only_identity() {
        use AssignmentCompatibility::{Identity as I, Unsupported as U};
        let matrix = [[I, U, U, U], [U, I, U, U], [U, U, I, U], [U, U, U, I]];
        for (row, target) in TYPES.into_iter().enumerate() {
            for (column, value) in TYPES.into_iter().enumerate() {
                assert_eq!(assignment(target, value), matrix[row][column]);
            }
        }
    }
    #[test]
    fn complete_unary_matrix() {
        let not = [Some(PlcType::Bool), None, None, None];
        let negate = [None, Some(PlcType::Int), Some(PlcType::Real), None];
        for (index, t) in TYPES.into_iter().enumerate() {
            assert_eq!(unary_result(UnaryOperator::Not, t), not[index]);
            assert_eq!(unary_result(UnaryOperator::Negate, t), negate[index]);
        }
    }
    #[test]
    fn complete_binary_matrix_including_time_and_mixed_numeric_types() {
        use BinaryOperator::*;
        use PlcType::{Bool, Int, Real};
        let boolean = [
            [Some(Bool), None, None, None],
            [None; 4],
            [None; 4],
            [None; 4],
        ];
        let arithmetic = [
            [None; 4],
            [None, Some(Int), None, None],
            [None, None, Some(Real), None],
            [None; 4],
        ];
        let equality = [
            [Some(Bool), None, None, None],
            [None, Some(Bool), None, None],
            [None, None, Some(Bool), None],
            [None, None, None, Some(Bool)],
        ];
        let ordering = [
            [None; 4],
            [None, Some(Bool), None, None],
            [None, None, Some(Bool), None],
            [None, None, None, Some(Bool)],
        ];
        for (operators, matrix) in [
            (vec![And, Or], boolean),
            (vec![Add, Subtract, Multiply, Divide], arithmetic),
            (vec![Equal, NotEqual], equality),
            (vec![Less, LessEqual, Greater, GreaterEqual], ordering),
        ] {
            for op in operators {
                for (row, left) in TYPES.into_iter().enumerate() {
                    for (column, right) in TYPES.into_iter().enumerate() {
                        assert_eq!(
                            binary_result(op, left, right),
                            matrix[row][column],
                            "{op:?} {left:?} {right:?}"
                        );
                    }
                }
            }
        }
    }
}
