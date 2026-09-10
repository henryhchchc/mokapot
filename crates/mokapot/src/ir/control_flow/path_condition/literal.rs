use std::{fmt::Display, ops::Not};

use crate::ir::TryMapValues;

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

impl<P> Display for BooleanVariable<P>
where
    P: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Positive(predicate) => predicate.fmt(f),
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

impl<P, OUT> TryMapValues<OUT> for BooleanVariable<P>
where
    P: TryMapValues<OUT>,
{
    type Value = P::Value;
    type Mapped = BooleanVariable<P::Mapped>;

    fn try_map_values<E>(
        self,
        remap: impl FnMut(P::Value) -> Result<OUT, E>,
    ) -> Result<BooleanVariable<P::Mapped>, E> {
        match self {
            Self::Positive(value) => value.try_map_values(remap).map(BooleanVariable::Positive),
            Self::Negative(value) => value.try_map_values(remap).map(BooleanVariable::Negative),
        }
    }
}
