use crate::{
    ir::{
        self, ValueId,
        expression::{LiftedCondition as Condition, Predicate},
    },
    jvm::ConstantValue,
};

use super::BooleanVariable;

impl<T> BooleanVariable<Condition<T>> {
    /// Rewrites equivalent conditions into a single literal vocabulary.
    fn canonicalize(self) -> Self {
        match self {
            Self::Positive(condition) => canonicalize_condition(condition),
            Self::Negative(condition) => !canonicalize_condition(condition),
        }
    }
}

/// Rewrites branch conditions into canonical positive/negative literals.
fn canonicalize_condition<T>(condition: Condition<T>) -> BooleanVariable<Condition<T>> {
    use BooleanVariable::{Negative, Positive};
    use Condition::{
        Equal, GreaterThan, GreaterThanOrEqual, IsNegative, IsNonNegative, IsNonPositive,
        IsNonZero, IsNotNull, IsNull, IsPositive, IsZero, LessThan, LessThanOrEqual, NotEqual,
    };

    match condition {
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

impl<T, V> From<ir::expression::LiftedCondition<T>>
    for BooleanVariable<ir::expression::LiftedCondition<V>>
where
    V: From<T>,
{
    fn from(value: ir::expression::LiftedCondition<T>) -> Self {
        #[allow(clippy::enum_glob_use)]
        use Condition::*;

        let condition = match value {
            Equal(lhs, rhs) => Equal(lhs.into(), rhs.into()),
            NotEqual(lhs, rhs) => NotEqual(lhs.into(), rhs.into()),
            LessThan(lhs, rhs) => LessThan(lhs.into(), rhs.into()),
            LessThanOrEqual(lhs, rhs) => LessThanOrEqual(lhs.into(), rhs.into()),
            GreaterThan(lhs, rhs) => GreaterThan(lhs.into(), rhs.into()),
            GreaterThanOrEqual(lhs, rhs) => GreaterThanOrEqual(lhs.into(), rhs.into()),
            IsNull(value) => IsNull(value.into()),
            IsNotNull(value) => IsNotNull(value.into()),
            IsZero(value) => IsZero(value.into()),
            IsNonZero(value) => IsNonZero(value.into()),
            IsPositive(value) => IsPositive(value.into()),
            IsNegative(value) => IsNegative(value.into()),
            IsNonNegative(value) => IsNonNegative(value.into()),
            IsNonPositive(value) => IsNonPositive(value.into()),
        };
        BooleanVariable::Positive(condition).canonicalize()
    }
}

mod model {
    use super::ConstantValue;

    /// An operand or constant parameterized by the lifting operand representation.
    #[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, derive_more::Display)]
    pub enum Value<OP> {
        /// A value produced by the IR.
        Variable(OP),
        /// A JVM constant embedded in the condition.
        Constant(ConstantValue),
    }
}

/// A scalar SSA value or JVM constant referenced by a path predicate.
pub type Value = model::Value<ValueId>;
pub(crate) use model::Value as LiftedValue;

impl Predicate {
    pub(crate) fn uses(&self) -> std::collections::HashSet<ValueId> {
        use Condition::{
            Equal, GreaterThan, GreaterThanOrEqual, IsNegative, IsNonNegative, IsNonPositive,
            IsNonZero, IsNotNull, IsNull, IsPositive, IsZero, LessThan, LessThanOrEqual, NotEqual,
        };

        let values = match self {
            Equal(lhs, rhs)
            | NotEqual(lhs, rhs)
            | LessThan(lhs, rhs)
            | LessThanOrEqual(lhs, rhs)
            | GreaterThan(lhs, rhs)
            | GreaterThanOrEqual(lhs, rhs) => vec![lhs, rhs],
            IsNull(value) | IsNotNull(value) | IsZero(value) | IsNonZero(value)
            | IsPositive(value) | IsNegative(value) | IsNonNegative(value)
            | IsNonPositive(value) => vec![value],
        };
        values
            .into_iter()
            .filter_map(|value| match value {
                LiftedValue::Variable(value) => Some(value),
                LiftedValue::Constant(_) => None,
            })
            .copied()
            .collect()
    }
}

impl<OP> From<OP> for model::Value<OP> {
    fn from(value: OP) -> Self {
        Self::Variable(value)
    }
}

impl From<ConstantValue> for Value {
    fn from(value: ConstantValue) -> Self {
        Self::Constant(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_complements() {
        let not_equal: BooleanVariable<Condition<u8>> = Condition::NotEqual(1, 2).into();
        assert_eq!(not_equal, BooleanVariable::Negative(Condition::Equal(1, 2)));

        let non_zero: BooleanVariable<Condition<u8>> = Condition::IsNonZero(3).into();
        assert_eq!(non_zero, BooleanVariable::Negative(Condition::IsZero(3)));
    }

    #[test]
    fn canonicalizes_order_directions() {
        let greater_than: BooleanVariable<Condition<u8>> = Condition::GreaterThan(1, 2).into();
        assert_eq!(
            greater_than,
            BooleanVariable::Positive(Condition::LessThan(2, 1))
        );

        let less_than_or_equal: BooleanVariable<Condition<u8>> =
            Condition::LessThanOrEqual(1, 2).into();
        assert_eq!(
            less_than_or_equal,
            BooleanVariable::Negative(Condition::LessThan(2, 1))
        );
    }
}
