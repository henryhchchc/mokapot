use std::collections::HashSet;

use crate::ir::{TryMapValues, ValueId};

/// A condition that can be used in a conditional jump.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum Condition<OP = ValueId> {
    /// The two arguments are equal (i.e., `lhs == rhs`).
    #[display("{_0} == {_1}")]
    Equal(OP, OP),
    /// The two arguments are not equal (i.e., `lhs != rhs`).
    #[display("{_0} != {_1}")]
    NotEqual(OP, OP),
    /// The first argument is less than the second (i.e., `lhs < rhs`).
    #[display("{_0} < {_1}")]
    LessThan(OP, OP),
    /// The first argument is less than or equal to the second (i.e., `lhs <= rhs`).
    #[display("{_0} <= {_1}")]
    LessThanOrEqual(OP, OP),
    /// The first argument is greater than the second (i.e., `lhs > rhs`).
    #[display("{_0} > {_1}")]
    GreaterThan(OP, OP),
    /// The first argument is greater than or equal to the second (i.e., `lhs >= rhs`).
    #[display("{_0} >= {_1}")]
    GreaterThanOrEqual(OP, OP),
    /// The argument is null (i.e., `arg == null`).
    #[display("{_0} == null")]
    IsNull(OP),
    /// The argument is not null (i.e., `arg != null`).
    #[display("{_0} != null")]
    IsNotNull(OP),
    /// The argument is zero (i.e., `arg == 0`).
    #[display("{_0} == 0")]
    IsZero(OP),
    /// The argument is not zero (i.e., `arg != 0`).
    #[display("{_0} != 0")]
    IsNonZero(OP),
    /// The argument is positive (i.e., `arg > 0`).
    #[display("{_0} > 0")]
    IsPositive(OP),
    /// The argument is negative (i.e., `arg < 0`).
    #[display("{_0} < 0")]
    IsNegative(OP),
    /// The argument is non-negative (i.e., `arg >= 0`).
    #[display("{_0} >= 0")]
    IsNonNegative(OP),
    /// The argument is non-positive (i.e., `arg <= 0`).
    #[display("{_0} <= 0")]
    IsNonPositive(OP),
}

impl Condition {
    /// Returns the values used by the condition.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::Equal(a, b)
            | Self::NotEqual(a, b)
            | Self::LessThan(a, b)
            | Self::LessThanOrEqual(a, b)
            | Self::GreaterThan(a, b)
            | Self::GreaterThanOrEqual(a, b) => HashSet::from([*a, *b]),
            Self::IsNull(a)
            | Self::IsNotNull(a)
            | Self::IsZero(a)
            | Self::IsNonZero(a)
            | Self::IsPositive(a)
            | Self::IsNegative(a)
            | Self::IsNonNegative(a)
            | Self::IsNonPositive(a) => HashSet::from([*a]),
        }
    }
}

impl<OP, OUT> TryMapValues<OUT> for Condition<OP> {
    type Value = OP;
    type Mapped = Condition<OUT>;

    fn try_map_values<E>(
        self,
        mut remap: impl FnMut(OP) -> Result<OUT, E>,
    ) -> Result<Condition<OUT>, E> {
        Ok(match self {
            Self::Equal(lhs, rhs) => Condition::Equal(remap(lhs)?, remap(rhs)?),
            Self::NotEqual(lhs, rhs) => Condition::NotEqual(remap(lhs)?, remap(rhs)?),
            Self::LessThan(lhs, rhs) => Condition::LessThan(remap(lhs)?, remap(rhs)?),
            Self::LessThanOrEqual(lhs, rhs) => Condition::LessThanOrEqual(remap(lhs)?, remap(rhs)?),
            Self::GreaterThan(lhs, rhs) => Condition::GreaterThan(remap(lhs)?, remap(rhs)?),
            Self::GreaterThanOrEqual(lhs, rhs) => {
                Condition::GreaterThanOrEqual(remap(lhs)?, remap(rhs)?)
            }
            Self::IsNull(operand) => Condition::IsNull(remap(operand)?),
            Self::IsNotNull(operand) => Condition::IsNotNull(remap(operand)?),
            Self::IsZero(operand) => Condition::IsZero(remap(operand)?),
            Self::IsNonZero(operand) => Condition::IsNonZero(remap(operand)?),
            Self::IsPositive(operand) => Condition::IsPositive(remap(operand)?),
            Self::IsNegative(operand) => Condition::IsNegative(remap(operand)?),
            Self::IsNonNegative(operand) => Condition::IsNonNegative(remap(operand)?),
            Self::IsNonPositive(operand) => Condition::IsNonPositive(remap(operand)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn check_uses(cond: &Condition, ids: &HashSet<ValueId>) {
        let cond_ids = cond.uses();
        for id in ids {
            assert!(cond_ids.contains(id));
        }
    }

    proptest! {


        #[test]
        fn uses(
            arg1 in any::<ValueId>(),
            arg2 in any::<ValueId>(),
        ) {
            let arg1_ids = HashSet::from([arg1]);
            let both_arg_ids = HashSet::from([arg1, arg2]);

            let eq = Condition::Equal(arg1, arg2);
            check_uses(&eq, &both_arg_ids);

            let ne = Condition::NotEqual(arg1, arg2);
            check_uses(&ne, &both_arg_ids);

            let lt = Condition::LessThan(arg1, arg2);
            check_uses(&lt, &both_arg_ids);

            let le = Condition::LessThanOrEqual(arg1, arg2);
            check_uses(&le, &both_arg_ids);

            let gt = Condition::GreaterThan(arg1, arg2);
            check_uses(&gt, &both_arg_ids);

            let ge = Condition::GreaterThanOrEqual(arg1, arg2);
            check_uses(&ge, &both_arg_ids);

            let is_null = Condition::IsNull(arg1);
            check_uses(&is_null, &arg1_ids);

            let is_not_null = Condition::IsNotNull(arg1);
            check_uses(&is_not_null, &arg1_ids);

            let is_zero = Condition::IsZero(arg1);
            check_uses(&is_zero, &arg1_ids);

            let is_non_zero = Condition::IsNonZero(arg1);
            check_uses(&is_non_zero, &arg1_ids);

            let is_positive = Condition::IsPositive(arg1);
            check_uses(&is_positive, &arg1_ids);

            let is_negative = Condition::IsNegative(arg1);
            check_uses(&is_negative, &arg1_ids);

            let is_non_negative = Condition::IsNonNegative(arg1);
            check_uses(&is_non_negative, &arg1_ids);

            let is_non_positive = Condition::IsNonPositive(arg1);
            check_uses(&is_non_positive, &arg1_ids);
        }
    }
}
