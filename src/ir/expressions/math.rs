use std::fmt::{Display, Formatter};

use crate::ir::ValueRef;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MathOperation {
    Add(ValueRef, ValueRef),
    Subtract(ValueRef, ValueRef),
    Multiply(ValueRef, ValueRef),
    Divide(ValueRef, ValueRef),
    Remainder(ValueRef, ValueRef),
    Negate(ValueRef),
    Increment(ValueRef),
    ShiftLeft(ValueRef, ValueRef),
    ShiftRight(ValueRef, ValueRef),
    LogicalShiftRight(ValueRef, ValueRef),
    BitwiseAnd(ValueRef, ValueRef),
    BitwiseOr(ValueRef, ValueRef),
    BitwiseXor(ValueRef, ValueRef),
    LongComparison(ValueRef, ValueRef),
    FloatingPointComparison(ValueRef, ValueRef, NaNTreatment),
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
