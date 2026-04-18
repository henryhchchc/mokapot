use std::{fmt::Display, ops::Not};

/// A variable in a path condition.
///
/// Represents either a positive or negative occurrence of a predicate.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum BooleanVariable<P> {
    /// A positive variable.
    Positive(P),
    /// A negative variable.
    Negative(P),
}

impl<P> BooleanVariable<P> {
    /// Returns a reference to the inner predicate of the variable.
    pub const fn predicate(&self) -> &P {
        match self {
            Self::Negative(predicate) | Self::Positive(predicate) => predicate,
        }
    }
}

impl<P> Display for BooleanVariable<P>
where
    P: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Positive(predicate) => write!(f, "({predicate})"),
            Self::Negative(predicate) => write!(f, "~({predicate})"),
        }
    }
}

impl<P> Not for BooleanVariable<P> {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Positive(predicate) => Self::Negative(predicate),
            Self::Negative(predicate) => Self::Positive(predicate),
        }
    }
}
