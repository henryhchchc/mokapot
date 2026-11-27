use derive_more::Display;

use super::{ValueId, expression::Expression};

/// An ordinary non-parameter, non-terminator operation.
#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum Operation<'class> {
    /// Evaluates an expression and defines its result.
    #[display("{_0} = {_1}")]
    Definition(ValueId, Expression<'class>),
    /// Evaluates an expression solely for its effects.
    #[display("{_0}")]
    Effect(Expression<'class>),
}
