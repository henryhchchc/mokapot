use std::collections::HashSet;

use crate::ir::ValueId;

/// A mathematical operation.
#[derive(Debug, PartialEq, Eq, Clone, derive_more::Display)]
pub enum Operation<OP: std::fmt::Display = ValueId> {
    /// Adds the two arguments (i.e., `lhs + rhs`).
    #[display("{_0} + {_1}")]
    Add(OP, OP),
    /// Subtracts the second argument from the first (i.e., `lhs - rhs`).
    #[display("{_0} - {_1}")]
    Subtract(OP, OP),
    /// Multiplies the two arguments (i.e., `lhs * rhs`).
    #[display("{_0} * {_1}")]
    Multiply(OP, OP),
    /// Divides the first argument by the second (i.e., `lhs / rhs`).
    #[display("{_0} / {_1}")]
    Divide(OP, OP),
    /// Computes the remainder of the first argument divided by the second (i.e., `lhs mod rhs`).
    #[display("{_0} mod {_1}")]
    Remainder(OP, OP),
    /// Negates the argument (i.e., `-arg`).
    #[display("-{_0}")]
    Negate(OP),
    /// Increments the argument by a constant (i.e., `arg + N`).
    #[display("{_0} + {_1}")]
    Increment(OP, i32),
    /// Shifts the first argument left by the second (i.e., `lhs << rhs`).
    #[display("{_0} << {_1}")]
    ShiftLeft(OP, OP),
    /// Shifts the first argument right by the second (i.e., `lhs >> rhs`).
    #[display("{_0} >> {_1}")]
    ShiftRight(OP, OP),
    /// Shifts the first argument right by the second, filling the leftmost bits with zeros (i.e., `lhs >>> rhs`).
    #[display("{_0} >>> {_1}")]
    LogicalShiftRight(OP, OP),
    /// Computes the bitwise AND of the two arguments (i.e., `lhs & rhs`).
    #[display("{_0} & {_1}")]
    BitwiseAnd(OP, OP),
    /// Computes the bitwise OR of the two arguments (i.e., `lhs | rhs`).
    #[display("{_0} | {_1}")]
    BitwiseOr(OP, OP),
    /// Computes the bitwise XOR of the two arguments (i.e., `lhs ^ rhs`).
    #[display("{_0} ^ {_1}")]
    BitwiseXor(OP, OP),
    /// Compares the two arguments as longs (i.e., `lhs lcmp rhs`).
    #[display("cmp({_0}, {_1})")]
    LongComparison(OP, OP),
    /// Compares the two arguments as floating point numbers (i.e., `lhs fcmp rhs`).
    #[display("cmp({_0}, {_1}) with {_2}")]
    FloatingPointComparison(OP, OP, NaNTreatment),
}
impl Operation<ValueId> {
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
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum NaNTreatment {
    /// NaNs are treated as the largest possible value.
    #[display("NaN == Max")]
    IsLargest,
    /// NaNs are treated as the smallest possible value.
    #[display("NaN == Min")]
    IsSmallest,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn uses(
            arg1 in any::<ValueId>(),
            arg2 in any::<ValueId>(),
            num in any::<i32>(),
            nan_treatment in any::<NaNTreatment>()
        ) {
            let bin_ops = [
                Operation::Add(arg1, arg2),
                Operation::Subtract(arg1, arg2),
                Operation::Multiply(arg1, arg2),
                Operation::Divide(arg1, arg2),
                Operation::Remainder(arg1, arg2),
                Operation::ShiftLeft(arg1, arg2),
                Operation::ShiftRight(arg1, arg2),
                Operation::LogicalShiftRight(arg1, arg2),
                Operation::BitwiseAnd(arg1, arg2),
                Operation::BitwiseOr(arg1, arg2),
                Operation::BitwiseXor(arg1, arg2),
                Operation::LongComparison(arg1, arg2),
                Operation::FloatingPointComparison(arg1, arg2, nan_treatment.clone()),
            ];
            let bin_ops_ids = HashSet::from([arg1, arg2]);
            for op in &bin_ops {
                assert_eq!(op.uses(), bin_ops_ids);
            }

            let unitary_ops = [
                Operation::Negate(arg1),
                Operation::Increment(arg1, num),
            ];
            let unitary_ops_ids = HashSet::from([arg1]);
            for op in &unitary_ops {
                assert_eq!(op.uses(), unitary_ops_ids);
            }
        }
    }
}
