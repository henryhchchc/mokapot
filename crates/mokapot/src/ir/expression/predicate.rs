use derive_more::{Display, From};

use super::{BooleanVariable, ValueId};
use crate::jvm::ConstantVal;

/// A branch predicate over SSA values and JVM constants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum Predicate<'class> {
    /// The two arguments are equal.
    #[display("{_0} == {_1}")]
    Equal(PathValue<'class>, PathValue<'class>),
    /// The two arguments are not equal.
    #[display("{_0} != {_1}")]
    NotEqual(PathValue<'class>, PathValue<'class>),
    /// The first argument is less than the second.
    #[display("{_0} < {_1}")]
    LessThan(PathValue<'class>, PathValue<'class>),
    /// The first argument is less than or equal to the second.
    #[display("{_0} <= {_1}")]
    LessThanOrEqual(PathValue<'class>, PathValue<'class>),
    /// The first argument is greater than the second.
    #[display("{_0} > {_1}")]
    GreaterThan(PathValue<'class>, PathValue<'class>),
    /// The first argument is greater than or equal to the second.
    #[display("{_0} >= {_1}")]
    GreaterThanOrEqual(PathValue<'class>, PathValue<'class>),
    /// The argument is null.
    #[display("{_0} == null")]
    IsNull(PathValue<'class>),
    /// The argument is not null.
    #[display("{_0} != null")]
    IsNotNull(PathValue<'class>),
    /// The argument is zero.
    #[display("{_0} == 0")]
    IsZero(PathValue<'class>),
    /// The argument is not zero.
    #[display("{_0} != 0")]
    IsNonZero(PathValue<'class>),
    /// The argument is positive.
    #[display("{_0} > 0")]
    IsPositive(PathValue<'class>),
    /// The argument is negative.
    #[display("{_0} < 0")]
    IsNegative(PathValue<'class>),
    /// The argument is non-negative.
    #[display("{_0} >= 0")]
    IsNonNegative(PathValue<'class>),
    /// The argument is non-positive.
    #[display("{_0} <= 0")]
    IsNonPositive(PathValue<'class>),
}

impl BooleanVariable<Predicate<'_>> {
    fn canonicalize(self) -> Self {
        match self {
            Self::Positive(predicate) => canonicalize_predicate(predicate),
            Self::Negative(predicate) => !canonicalize_predicate(predicate),
        }
    }
}

const fn canonicalize_predicate(predicate: Predicate<'_>) -> BooleanVariable<Predicate<'_>> {
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

impl<'class> From<Predicate<'class>> for BooleanVariable<Predicate<'class>> {
    fn from(predicate: Predicate<'class>) -> Self {
        BooleanVariable::Positive(predicate).canonicalize()
    }
}

/// An SSA value or constant in a path predicate.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Display, From)]
pub enum PathValue<'class> {
    /// A value produced by the IR.
    Variable(#[from] ValueId),
    /// A JVM constant embedded in the condition.
    Constant(#[from] ConstantVal<'class>),
}

impl<'class> Predicate<'class> {
    pub(crate) fn map_values(
        &self,
        mut map: impl FnMut(&PathValue<'class>) -> PathValue<'class>,
    ) -> Self {
        let mut mapped = self.clone();
        let Ok(()) = mapped.try_for_each_value_mut(|value| {
            *value = map(value);
            Ok::<_, !>(())
        });
        mapped
    }

    pub(crate) fn try_for_each_value_mut<E>(
        &mut self,
        mut visit: impl FnMut(&mut PathValue<'class>) -> Result<(), E>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test::prelude::ids;

    #[test]
    fn conversion_canonicalizes_comparison_and_complement_predicates() {
        let [x_id, y_id] = ids(0);
        let x = PathValue::Variable(x_id);
        let y = PathValue::Variable(y_id);
        assert_eq!(
            BooleanVariable::from(Predicate::GreaterThan(x.clone(), y.clone())),
            BooleanVariable::Positive(Predicate::LessThan(y.clone(), x.clone()))
        );
        assert_eq!(
            BooleanVariable::from(Predicate::GreaterThanOrEqual(x.clone(), y.clone())),
            BooleanVariable::Negative(Predicate::LessThan(x.clone(), y.clone()))
        );
        assert_eq!(
            BooleanVariable::from(Predicate::LessThanOrEqual(x.clone(), y.clone())),
            BooleanVariable::Negative(Predicate::LessThan(y.clone(), x.clone()))
        );
        assert_eq!(
            BooleanVariable::from(Predicate::NotEqual(x.clone(), y.clone())),
            BooleanVariable::Negative(Predicate::Equal(x.clone(), y.clone()))
        );
        assert_eq!(
            BooleanVariable::from(Predicate::IsNonZero(x.clone())),
            BooleanVariable::Negative(Predicate::IsZero(x.clone()))
        );
        assert_eq!(
            BooleanVariable::from(Predicate::IsNotNull(x.clone())),
            BooleanVariable::Negative(Predicate::IsNull(x.clone()))
        );
    }
}
