use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
};

use crate::ir::{Argument, Identifier};

/// A mathematical operation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Operation {
    /// Adds the two arguments (i.e., `lhs + rhs``).
    Add(Argument, Argument),
    /// Subtracts the second argument from the first (i.e., `lhs - rhs`).
    Subtract(Argument, Argument),
    /// Multiplies the two arguments (i.e., `lhs * rhs`).
    Multiply(Argument, Argument),
    /// Divides the first argument by the second (i.e., `lhs / rhs`).
    Divide(Argument, Argument),
    /// Computes the remainder of the first argument divided by the second (i.e., `lhs mod rhs`).
    Remainder(Argument, Argument),
    /// Negates the argument (i.e., `-arg`).
    Negate(Argument),
    /// Increments the argument by a constant (i.e., `arg + N`).
    Increment(Argument, i32),
    /// Shifts the first argument left by the second (i.e., `lhs << rhs`).
    ShiftLeft(Argument, Argument),
    /// Shifts the first argument right by the second (i.e., `lhs >> rhs`).
    ShiftRight(Argument, Argument),
    /// Shifts the first argument right by the second, filling the leftmost bits with zeros (i.e., `lhs >>> rhs`).
    LogicalShiftRight(Argument, Argument),
    /// Computes the bitwise AND of the two arguments (i.e., `lhs & rhs`).
    BitwiseAnd(Argument, Argument),
    /// Computes the bitwise OR of the two arguments (i.e., `lhs | rhs`).
    BitwiseOr(Argument, Argument),
    /// Computes the bitwise XOR of the two arguments (i.e., `lhs ^ rhs`).
    BitwiseXor(Argument, Argument),
    /// Compares the two arguments as longs (i.e., `lhs lcmp rhs`).
    LongComparison(Argument, Argument),
    /// Compares the two arguments as floating point numbers (i.e., `lhs fcmp rhs`).
    FloatingPointComparison(Argument, Argument, NaNTreatment),
}
impl Operation {
    /// Returns the set of [`Identifier`]s used by the expression.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
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
            | Self::FloatingPointComparison(a, b, _) => a.iter().chain(b.iter()).copied().collect(),
            Self::Negate(a) | Self::Increment(a, _) => a.iter().copied().collect(),
        }
    }
}

/// How NaNs are treated in floating point comparisons.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum NaNTreatment {
    /// NaNs are treated as the largest possible value.
    IsLargest,
    /// NaNs are treated as the smallest possible value.
    IsSmallest,
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add(a, b) => write!(f, "{a} + {b}"),
            Self::Subtract(a, b) => write!(f, "{a} - {b}"),
            Self::Multiply(a, b) => write!(f, "{a} * {b}"),
            Self::Divide(a, b) => write!(f, "{a} / {b}"),
            Self::Remainder(a, b) => write!(f, "{a} mod {b}"),
            Self::Negate(a) => write!(f, "-{a}"),
            Self::Increment(a, n) => write!(f, "{a} + {n}"),
            Self::ShiftLeft(a, b) => write!(f, "{a} << {b}"),
            Self::ShiftRight(a, b) => write!(f, "{a} >> {b}"),
            Self::LogicalShiftRight(a, b) => write!(f, "{a} >>> {b}"),
            Self::BitwiseAnd(a, b) => write!(f, "{a} & {b}"),
            Self::BitwiseOr(a, b) => write!(f, "{a} | {b}"),
            Self::BitwiseXor(a, b) => write!(f, "{a} ^ {b}"),
            Self::LongComparison(a, b) => write!(f, "cmp({a}, {b})"),
            Self::FloatingPointComparison(a, b, NaNTreatment::IsLargest) => {
                write!(f, "cmp({a}, {b}) nan is largest")
            }
            Self::FloatingPointComparison(a, b, NaNTreatment::IsSmallest) => {
                write!(f, "cmp({a}, {b}) nan is smallest")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::test::arb_argument;

    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn uses(
            arg1 in arb_argument(),
            arg2 in arb_argument(),
            num in any::<i32>(),
            nan_treament in any::<NaNTreatment>()
        ) {
            let bin_ops = [
                Operation::Add(arg1.clone(), arg2.clone()),
                Operation::Subtract(arg1.clone(), arg2.clone()),
                Operation::Multiply(arg1.clone(), arg2.clone()),
                Operation::Divide(arg1.clone(), arg2.clone()),
                Operation::Remainder(arg1.clone(), arg2.clone()),
                Operation::ShiftLeft(arg1.clone(), arg2.clone()),
                Operation::ShiftRight(arg1.clone(), arg2.clone()),
                Operation::LogicalShiftRight(arg1.clone(), arg2.clone()),
                Operation::BitwiseAnd(arg1.clone(), arg2.clone()),
                Operation::BitwiseOr(arg1.clone(), arg2.clone()),
                Operation::BitwiseXor(arg1.clone(), arg2.clone()),
                Operation::LongComparison(arg1.clone(), arg2.clone()),
                Operation::FloatingPointComparison(arg1.clone(), arg2.clone(), nan_treament.clone()),
            ];
            let bin_ops_ids = arg1.iter().chain(arg2.iter()).copied().collect::<BTreeSet<_>>();
            for op in &bin_ops {
                assert_eq!(op.uses(), bin_ops_ids);
            }

            let unitary_ops = [
                Operation::Negate(arg1.clone()),
                Operation::Increment(arg1.clone(), num),
            ];
            let unitary_ops_ids = arg1.iter().copied().collect::<BTreeSet<_>>();
            for op in &unitary_ops {
                assert_eq!(op.uses(), unitary_ops_ids);
            }
        }
    }
}
