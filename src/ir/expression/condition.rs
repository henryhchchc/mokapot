use std::fmt::Display;

use crate::ir::Argument;

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Equal(Argument, Argument),
    NotEqual(Argument, Argument),
    LessThan(Argument, Argument),
    LessThanOrEqual(Argument, Argument),
    GreaterThan(Argument, Argument),
    GreaterThanOrEqual(Argument, Argument),
    IsNull(Argument),
    IsNotNull(Argument),
    Zero(Argument),
    NonZero(Argument),
    Positive(Argument),
    Negative(Argument),
    NonNegative(Argument),
    NonPositive(Argument),
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
            Zero(a) => write!(f, "{} == 0", a),
            NonZero(a) => write!(f, "{} != 0", a),
            Positive(a) => write!(f, "{} > 0", a),
            Negative(a) => write!(f, "{} < 0", a),
            NonNegative(a) => write!(f, "{} >= 0", a),
            NonPositive(a) => write!(f, "{} <= 0", a),
        }
    }
}
