use crate::ir::control_flow::path_condition::PathValue;

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
