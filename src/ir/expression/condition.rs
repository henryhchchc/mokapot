use std::fmt::Display;

use crate::ir::Argument;

/// A condition that can be used in a conditional jump.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// The two arguments are equal (i.e., `lhs == rhs`).
    Equal(Argument, Argument),
    /// The two arguments are not equal (i.e., `lhs != rhs`).
    NotEqual(Argument, Argument),
    /// The first argument is less than the second (i.e., `lhs < rhs`).
    LessThan(Argument, Argument),
    /// The first argument is less than or equal to the second (i.e., `lhs <= rhs`).
    LessThanOrEqual(Argument, Argument),
    /// The first argument is greater than the second (i.e., `lhs > rhs`).
    GreaterThan(Argument, Argument),
    /// The first argument is greater than or equal to the second (i.e., `lhs >= rhs`).
    GreaterThanOrEqual(Argument, Argument),
    /// The argument is null (i.e., `arg == null`).
    IsNull(Argument),
    /// The argument is not null (i.e., `arg != null`).
    IsNotNull(Argument),
    /// The argument is zero (i.e., `arg == 0`).
    IsZero(Argument),
    /// The argument is not zero (i.e., `arg != 0`).
    IsNonZero(Argument),
    /// The argument is positive (i.e., `arg > 0`).
    IsPositive(Argument),
    /// The argument is negative (i.e., `arg < 0`).
    IsNegative(Argument),
    /// The argument is non-negative (i.e., `arg >= 0`).
    IsNonNegative(Argument),
    /// The argument is non-positive (i.e., `arg <= 0`).
    IsNonPositive(Argument),
}

impl Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Condition::*;
        match self {
            Equal(a, b) => write!(f, "{} == {}", a, b),
            NotEqual(a, b) => write!(f, "{} != {}", a, b),
            LessThan(a, b) => write!(f, "{} < {}", a, b),
            LessThanOrEqual(a, b) => write!(f, "{} <= {}", a, b),
            GreaterThan(a, b) => write!(f, "{} > {}", a, b),
            GreaterThanOrEqual(a, b) => write!(f, "{} >= {}", a, b),
            IsNull(a) => write!(f, "{} == null", a),
            IsNotNull(a) => write!(f, "{} != null", a),
            IsZero(a) => write!(f, "{} == 0", a),
            IsNonZero(a) => write!(f, "{} != 0", a),
            IsPositive(a) => write!(f, "{} > 0", a),
            IsNegative(a) => write!(f, "{} < 0", a),
            IsNonNegative(a) => write!(f, "{} >= 0", a),
            IsNonPositive(a) => write!(f, "{} <= 0", a),
        }
    }
}
