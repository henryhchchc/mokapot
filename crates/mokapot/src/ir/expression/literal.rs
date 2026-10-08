//! Signed predicates used as boolean literals in conditions.

use std::ops::Not;

use derive_more::Display;

/// A variable in a path condition: a positive or negative occurrence of a
/// predicate.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Display)]
pub enum BooleanVariable<P> {
    /// A positive variable.
    #[display("{_0}")]
    Positive(P),
    /// A negative variable.
    #[display("~({_0})")]
    Negative(P),
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
