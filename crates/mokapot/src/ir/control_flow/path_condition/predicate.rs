use crate::{
    ir::{self, Operand, expression::Condition},
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

impl<T, V> From<ir::expression::Condition<T>> for BooleanVariable<ir::expression::Condition<V>>
where
    V: From<T>,
{
    fn from(value: ir::expression::Condition<T>) -> Self {
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

/// An operand or constant referenced by a path predicate.
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, derive_more::Display)]
pub enum Value {
    /// A value produced by the IR.
    Variable(Operand),
    /// A JVM constant embedded in the condition.
    Constant(ConstantValue),
}

impl From<ir::Operand> for Value {
    fn from(value: ir::Operand) -> Self {
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
