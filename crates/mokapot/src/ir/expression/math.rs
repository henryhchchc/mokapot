use std::collections::HashSet;

use super::ValueId;

/// A mathematical operation.
#[derive(Debug, PartialEq, Eq, Clone, derive_more::Display)]
pub enum Operation {
    /// Adds the two arguments (i.e., `lhs + rhs`).
    #[display("{_0} + {_1}")]
    Add(ValueId, ValueId),
    /// Subtracts the second argument from the first (i.e., `lhs - rhs`).
    #[display("{_0} - {_1}")]
    Subtract(ValueId, ValueId),
    /// Multiplies the two arguments (i.e., `lhs * rhs`).
    #[display("{_0} * {_1}")]
    Multiply(ValueId, ValueId),
    /// Divides the first argument by the second (i.e., `lhs / rhs`).
    #[display("{_0} / {_1}")]
    Divide(ValueId, ValueId),
    /// Computes the remainder of the first argument divided by the second (i.e., `lhs mod rhs`).
    #[display("{_0} mod {_1}")]
    Remainder(ValueId, ValueId),
    /// Negates the argument (i.e., `-arg`).
    #[display("-{_0}")]
    Negate(ValueId),
    /// Increments the argument by a constant (i.e., `arg + N`).
    #[display("{_0} + {_1}")]
    Increment(ValueId, i32),
    /// Shifts the first argument left by the second (i.e., `lhs << rhs`).
    #[display("{_0} << {_1}")]
    ShiftLeft(ValueId, ValueId),
    /// Shifts the first argument right by the second (i.e., `lhs >> rhs`).
    #[display("{_0} >> {_1}")]
    ShiftRight(ValueId, ValueId),
    /// Shifts the first argument right by the second, filling the leftmost bits with zeros (i.e., `lhs >>> rhs`).
    #[display("{_0} >>> {_1}")]
    LogicalShiftRight(ValueId, ValueId),
    /// Computes the bitwise AND of the two arguments (i.e., `lhs & rhs`).
    #[display("{_0} & {_1}")]
    BitwiseAnd(ValueId, ValueId),
    /// Computes the bitwise OR of the two arguments (i.e., `lhs | rhs`).
    #[display("{_0} | {_1}")]
    BitwiseOr(ValueId, ValueId),
    /// Computes the bitwise XOR of the two arguments (i.e., `lhs ^ rhs`).
    #[display("{_0} ^ {_1}")]
    BitwiseXor(ValueId, ValueId),
    /// Compares the two arguments as longs (i.e., `lhs lcmp rhs`).
    #[display("cmp({_0}, {_1})")]
    LongComparison(ValueId, ValueId),
    /// Compares the two arguments as floating point numbers (i.e., `lhs fcmp rhs`).
    #[display("cmp({_0}, {_1}) with {_2}")]
    FloatingPointComparison(ValueId, ValueId, NaNTreatment),
}
impl Operation {
    /// Returns the values used by the expression.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::Add(a, b)
            | Self::Subtract(a, b)
            | Self::Multiply(a, b)
            | Self::Divide(a, b)
            | Self::Remainder(a, b)
            | Self::ShiftLeft(a, b)
            | Self::ShiftRight(a, b)
            | Self::LogicalShiftRight(a, b)
            | Self::BitwiseAnd(a, b)
            | Self::BitwiseOr(a, b)
            | Self::BitwiseXor(a, b)
            | Self::LongComparison(a, b)
            | Self::FloatingPointComparison(a, b, _) => HashSet::from([*a, *b]),
            Self::Negate(a) | Self::Increment(a, _) => HashSet::from([*a]),
        }
    }
}

/// How NaNs are treated in floating point comparisons.
#[derive(Debug, PartialEq, Eq, Clone, derive_more::Display)]
pub enum NaNTreatment {
    /// NaNs are treated as the largest possible value.
    #[display("NaN == Max")]
    IsLargest,
    /// NaNs are treated as the smallest possible value.
    #[display("NaN == Min")]
    IsSmallest,
}
