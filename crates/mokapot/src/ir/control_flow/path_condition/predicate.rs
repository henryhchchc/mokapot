use std::collections::HashSet;

use crate::{
    ir::{ValueId, expression::Predicate},
    jvm::ConstantValue,
};

use super::BooleanVariable;

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
    pub(crate) fn uses(&self) -> HashSet<ValueId> {
        use Predicate::{
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
                PathValue::Variable(value) => Some(value),
                PathValue::Constant(_) => None,
            })
            .copied()
            .collect()
    }
}
