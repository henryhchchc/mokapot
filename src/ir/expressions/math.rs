use std::fmt::{Display, Formatter};

use crate::ir::Argument;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MathOperation {
    Add(Argument, Argument),
    Subtract(Argument, Argument),
    Multiply(Argument, Argument),
    Divide(Argument, Argument),
    Remainder(Argument, Argument),
    Negate(Argument),
    Increment(Argument),
    ShiftLeft(Argument, Argument),
    ShiftRight(Argument, Argument),
    LogicalShiftRight(Argument, Argument),
    BitwiseAnd(Argument, Argument),
    BitwiseOr(Argument, Argument),
    BitwiseXor(Argument, Argument),
    LongComparison(Argument, Argument),
    FloatingPointComparison(Argument, Argument, NaNTreatment),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NaNTreatment {
    IsLargest,
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
