use std::fmt::{Display, Formatter};

use crate::ir::Argument;

/// A mathematical operation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MathOperation {
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
    /// Increments the argument by one (i.e., `arg + 1`).
    Increment(Argument),
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

/// How NaNs are treated in floating point comparisons.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NaNTreatment {
    /// NaNs are treated as the largest possible value.
    IsLargest,
    /// NaNs are treated as the smallest possible value.
    IsSmallest,
}

impl Display for MathOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use MathOperation::*;
        match self {
            Add(a, b) => write!(f, "{} + {}", a, b),
            Subtract(a, b) => write!(f, "{} - {}", a, b),
            Multiply(a, b) => write!(f, "{} * {}", a, b),
            Divide(a, b) => write!(f, "{} / {}", a, b),
            Remainder(a, b) => write!(f, "{} % {}", a, b),
            Negate(a) => write!(f, "-{}", a),
            Increment(a) => write!(f, "{} + 1", a),
            ShiftLeft(a, b) => write!(f, "{} << {}", a, b),
            ShiftRight(a, b) => write!(f, "{} >> {}", a, b),
            LogicalShiftRight(a, b) => write!(f, "{} >>> {}", a, b),
            BitwiseAnd(a, b) => write!(f, "{} & {}", a, b),
            BitwiseOr(a, b) => write!(f, "{} | {}", a, b),
            BitwiseXor(a, b) => write!(f, "{} ^ {}", a, b),
            LongComparison(a, b) => write!(f, "cmp({}, {})", a, b),
            FloatingPointComparison(a, b, NaNTreatment::IsLargest) => {
                write!(f, "cmp({}, {}) nan is largest", a, b)
            }
            FloatingPointComparison(a, b, NaNTreatment::IsSmallest) => {
                write!(f, "cmp({}, {}) nan is smallest", a, b)
            }
        }
    }
}
