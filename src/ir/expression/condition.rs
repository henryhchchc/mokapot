use std::collections::BTreeSet;

use crate::ir::{Identifier, Operand};

/// A condition that can be used in a conditional jump.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
#[instability::unstable(feature = "moka-ir")]
pub enum Condition<OP = Operand> {
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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::ir::test::arb_argument;

    fn check_uses(cond: &Condition, ids: &BTreeSet<Identifier>) {
        let cond_ids = cond.uses();
        for id in ids {
            assert!(cond_ids.contains(id));
        }
    }

    proptest! {


        #[test]
        fn uses(
            arg1 in arb_argument(),
            arg2 in arb_argument(),
        ) {
            let arg1_ids = arg1.clone().into_iter().collect();
            let both_arg_ids = arg1.iter().chain(arg2.iter()).copied().collect();

            let eq = Condition::Equal(arg1.clone(), arg2.clone());
            check_uses(&eq, &both_arg_ids);

            let ne = Condition::NotEqual(arg1.clone(), arg2.clone());
            check_uses(&ne, &both_arg_ids);

            let lt = Condition::LessThan(arg1.clone(), arg2.clone());
            check_uses(&lt, &both_arg_ids);

            let le = Condition::LessThanOrEqual(arg1.clone(), arg2.clone());
            check_uses(&le, &both_arg_ids);

            let gt = Condition::GreaterThan(arg1.clone(), arg2.clone());
            check_uses(&gt, &both_arg_ids);

            let ge = Condition::GreaterThanOrEqual(arg1.clone(), arg2.clone());
            check_uses(&ge, &both_arg_ids);

            let is_null = Condition::IsNull(arg1.clone());
            check_uses(&is_null, &arg1_ids);

            let is_not_null = Condition::IsNotNull(arg1.clone());
            check_uses(&is_not_null, &arg1_ids);

            let is_zero = Condition::IsZero(arg1.clone());
            check_uses(&is_zero, &arg1_ids);

            let is_non_zero = Condition::IsNonZero(arg1.clone());
            check_uses(&is_non_zero, &arg1_ids);

            let is_positive = Condition::IsPositive(arg1.clone());
            check_uses(&is_positive, &arg1_ids);

            let is_negative = Condition::IsNegative(arg1.clone());
            check_uses(&is_negative, &arg1_ids);

            let is_non_negative = Condition::IsNonNegative(arg1.clone());
            check_uses(&is_non_negative, &arg1_ids);

            let is_non_positive = Condition::IsNonPositive(arg1.clone());
            check_uses(&is_non_positive, &arg1_ids);
        }
    }
}
