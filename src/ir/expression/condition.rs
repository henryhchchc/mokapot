use std::{collections::BTreeSet, fmt::Display};

use crate::ir::{Argument, Identifier};

/// A condition that can be used in a conditional jump.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl Condition {
    /// Returns the set of [`Identifier`]s used by the condition.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::Equal(a, b)
            | Self::NotEqual(a, b)
            | Self::LessThan(a, b)
            | Self::LessThanOrEqual(a, b)
            | Self::GreaterThan(a, b)
            | Self::GreaterThanOrEqual(a, b) => a.iter().chain(b.iter()).copied().collect(),
            Self::IsNull(a)
            | Self::IsNotNull(a)
            | Self::IsZero(a)
            | Self::IsNonZero(a)
            | Self::IsPositive(a)
            | Self::IsNegative(a)
            | Self::IsNonNegative(a)
            | Self::IsNonPositive(a) => a.iter().copied().collect(),
        }
    }
}

impl Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal(a, b) => write!(f, "{a} == {b}"),
            Self::NotEqual(a, b) => write!(f, "{a} != {b}"),
            Self::LessThan(a, b) => write!(f, "{a} < {b}"),
            Self::LessThanOrEqual(a, b) => write!(f, "{a} <= {b}"),
            Self::GreaterThan(a, b) => write!(f, "{a} > {b}"),
            Self::GreaterThanOrEqual(a, b) => write!(f, "{a} >= {b}"),
            Self::IsNull(a) => write!(f, "{a} == null"),
            Self::IsNotNull(a) => write!(f, "{a} != null"),
            Self::IsZero(a) => write!(f, "{a} == 0"),
            Self::IsNonZero(a) => write!(f, "{a} != 0"),
            Self::IsPositive(a) => write!(f, "{a} > 0"),
            Self::IsNegative(a) => write!(f, "{a} < 0"),
            Self::IsNonNegative(a) => write!(f, "{a} >= 0"),
            Self::IsNonPositive(a) => write!(f, "{a} <= 0"),
        }
    }
}
