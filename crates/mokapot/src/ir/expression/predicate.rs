use std::{collections::HashSet, convert::Infallible};

use super::{BooleanVariable, ValueId};
use crate::jvm::ConstantValue;

/// A branch predicate over SSA values and JVM constants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum Predicate {
    /// The two arguments are equal.
    #[display("{_0} == {_1}")]
    Equal(PathValue, PathValue),
    /// The two arguments are not equal.
    #[display("{_0} != {_1}")]
    NotEqual(PathValue, PathValue),
    /// The first argument is less than the second.
    #[display("{_0} < {_1}")]
    LessThan(PathValue, PathValue),
    /// The first argument is less than or equal to the second.
    #[display("{_0} <= {_1}")]
    LessThanOrEqual(PathValue, PathValue),
    /// The first argument is greater than the second.
    #[display("{_0} > {_1}")]
    GreaterThan(PathValue, PathValue),
    /// The first argument is greater than or equal to the second.
    #[display("{_0} >= {_1}")]
    GreaterThanOrEqual(PathValue, PathValue),
    /// The argument is null.
    #[display("{_0} == null")]
    IsNull(PathValue),
    /// The argument is not null.
    #[display("{_0} != null")]
    IsNotNull(PathValue),
    /// The argument is zero.
    #[display("{_0} == 0")]
    IsZero(PathValue),
    /// The argument is not zero.
    #[display("{_0} != 0")]
    IsNonZero(PathValue),
    /// The argument is positive.
    #[display("{_0} > 0")]
    IsPositive(PathValue),
    /// The argument is negative.
    #[display("{_0} < 0")]
    IsNegative(PathValue),
    /// The argument is non-negative.
    #[display("{_0} >= 0")]
    IsNonNegative(PathValue),
    /// The argument is non-positive.
    #[display("{_0} <= 0")]
    IsNonPositive(PathValue),
}

impl BooleanVariable<Predicate> {
    fn canonicalize(self) -> Self {
        match self {
            Self::Positive(predicate) => canonicalize_predicate(predicate),
            Self::Negative(predicate) => !canonicalize_predicate(predicate),
        }
    }
}

fn canonicalize_predicate(predicate: Predicate) -> BooleanVariable<Predicate> {
    use BooleanVariable::{Negative, Positive};
    use Predicate::{
        Equal, GreaterThan, GreaterThanOrEqual, IsNegative, IsNonNegative, IsNonPositive,
        IsNonZero, IsNotNull, IsNull, IsPositive, IsZero, LessThan, LessThanOrEqual, NotEqual,
    };

    match predicate {
        Equal(lhs, rhs) => Positive(Equal(lhs, rhs)),
        NotEqual(lhs, rhs) => Negative(Equal(lhs, rhs)),
        LessThan(lhs, rhs) => Positive(LessThan(lhs, rhs)),
        LessThanOrEqual(lhs, rhs) => Negative(LessThan(rhs, lhs)),
        GreaterThan(lhs, rhs) => Positive(LessThan(rhs, lhs)),
        GreaterThanOrEqual(lhs, rhs) => Negative(LessThan(lhs, rhs)),
        IsNull(value) => Positive(IsNull(value)),
        IsNotNull(value) => Negative(IsNull(value)),
        IsZero(value) => Positive(IsZero(value)),
        IsNonZero(value) => Negative(IsZero(value)),
        IsPositive(value) => Positive(IsPositive(value)),
        IsNegative(value) => Positive(IsNegative(value)),
        IsNonNegative(value) => Negative(IsNegative(value)),
        IsNonPositive(value) => Negative(IsPositive(value)),
    }
}

impl From<Predicate> for BooleanVariable<Predicate> {
    fn from(predicate: Predicate) -> Self {
        BooleanVariable::Positive(predicate).canonicalize()
    }
}

/// An SSA value or constant in a path predicate.
#[derive(Debug, PartialEq, Eq, Clone, Hash, derive_more::Display)]
pub enum PathValue {
    /// A value produced by the IR.
    Variable(ValueId),
    /// A JVM constant embedded in the condition.
    Constant(ConstantValue),
}

impl From<ValueId> for PathValue {
    fn from(value: ValueId) -> Self {
        Self::Variable(value)
    }
}

impl From<ConstantValue> for PathValue {
    fn from(value: ConstantValue) -> Self {
        Self::Constant(value)
    }
}

impl Predicate {
    fn values(&self) -> impl Iterator<Item = &PathValue> {
        use Predicate::{
            Equal, GreaterThan, GreaterThanOrEqual, IsNegative, IsNonNegative, IsNonPositive,
            IsNonZero, IsNotNull, IsNull, IsPositive, IsZero, LessThan, LessThanOrEqual, NotEqual,
        };

        let (first, second) = match self {
            Equal(lhs, rhs)
            | NotEqual(lhs, rhs)
            | LessThan(lhs, rhs)
            | LessThanOrEqual(lhs, rhs)
            | GreaterThan(lhs, rhs)
            | GreaterThanOrEqual(lhs, rhs) => (lhs, Some(rhs)),
            IsNull(value) | IsNotNull(value) | IsZero(value) | IsNonZero(value)
            | IsPositive(value) | IsNegative(value) | IsNonNegative(value)
            | IsNonPositive(value) => (value, None),
        };
        std::iter::once(first).chain(second)
    }

    pub(crate) fn uses(&self) -> HashSet<ValueId> {
        self.values()
            .filter_map(|value| match value {
                PathValue::Variable(value) => Some(value),
                PathValue::Constant(_) => None,
            })
            .copied()
            .collect()
    }

    pub(crate) fn map_values(&self, mut map: impl FnMut(&PathValue) -> PathValue) -> Self {
        let mut mapped = self.clone();
        let Ok(()) = mapped.try_for_each_value_mut(|value| {
            *value = map(value);
            Ok::<_, Infallible>(())
        });
        mapped
    }

    pub(crate) fn try_for_each_value_mut<E>(
        &mut self,
        mut visit: impl FnMut(&mut PathValue) -> Result<(), E>,
    ) -> Result<(), E> {
        use Predicate::{
            Equal, GreaterThan, GreaterThanOrEqual, IsNegative, IsNonNegative, IsNonPositive,
            IsNonZero, IsNotNull, IsNull, IsPositive, IsZero, LessThan, LessThanOrEqual, NotEqual,
        };

        match self {
            Equal(lhs, rhs)
            | NotEqual(lhs, rhs)
            | LessThan(lhs, rhs)
            | LessThanOrEqual(lhs, rhs)
            | GreaterThan(lhs, rhs)
            | GreaterThanOrEqual(lhs, rhs) => {
                visit(lhs)?;
                visit(rhs)
            }
            IsNull(value) | IsNotNull(value) | IsZero(value) | IsNonZero(value)
            | IsPositive(value) | IsNegative(value) | IsNonNegative(value)
            | IsNonPositive(value) => visit(value),
        }
    }
}
