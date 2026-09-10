use std::{collections::HashSet, fmt};

use super::{InstructionId, TryMapValues, ValueId, expression::Expression};

/// The kind of an ordinary Moka IR operation.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum OperationKind<OP = ValueId> {
    /// Evaluates an expression and defines its result.
    #[display("{value} = {expr}")]
    Definition {
        /// The value defined by the expression.
        value: OP,
        /// The expression producing the value.
        expr: Expression<OP>,
    },
    /// Evaluates an expression solely for its effects.
    #[display("{expr}")]
    Effect {
        /// The effectful expression.
        expr: Expression<OP>,
    },
}

impl OperationKind<ValueId> {
    /// Returns the values used by this operation.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::Definition { expr, .. } | Self::Effect { expr } => expr.uses(),
        }
    }
}

impl<OP: Copy> OperationKind<OP> {
    /// Returns the value defined by this operation, if any.
    #[must_use]
    pub const fn def(&self) -> Option<OP> {
        match self {
            Self::Definition { value, .. } => Some(*value),
            Self::Effect { .. } => None,
        }
    }
}

impl<OP, OUT> TryMapValues<OUT> for OperationKind<OP> {
    type Value = OP;
    type Mapped = OperationKind<OUT>;

    fn try_map_values<E>(
        self,
        mut remap: impl FnMut(OP) -> Result<OUT, E>,
    ) -> Result<OperationKind<OUT>, E> {
        Ok(match self {
            Self::Definition { value, expr } => OperationKind::Definition {
                value: remap(value)?,
                expr: expr.try_map_values(remap)?,
            },
            Self::Effect { expr } => OperationKind::Effect {
                expr: expr.try_map_values(remap)?,
            },
        })
    }
}

/// An identified ordinary non-phi, non-terminator operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub(super) id: InstructionId,
    pub(super) kind: OperationKind,
}

impl Operation {
    /// Returns this operation's method-local instruction identity.
    #[must_use]
    pub const fn id(&self) -> InstructionId {
        self.id
    }
    /// Returns the kind of operation performed.
    #[must_use]
    pub const fn kind(&self) -> &OperationKind {
        &self.kind
    }
    /// Returns the value defined by this operation, if any.
    #[must_use]
    pub const fn def(&self) -> Option<ValueId> {
        self.kind.def()
    }
    /// Returns the values used by this operation.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        self.kind.uses()
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{OperationKind, TryMapValues};
    use crate::ir::expression::{Expression, MathOperation};

    #[test]
    fn maps_definition_and_expression_values() {
        let operation = OperationKind::Definition {
            value: 1_u8,
            expr: Expression::Math(MathOperation::Add(2, 3)),
        };

        assert_eq!(
            operation.try_map_values(|value| Ok::<_, ()>(u16::from(value) + 10)),
            Ok(OperationKind::Definition {
                value: 11_u16,
                expr: Expression::Math(MathOperation::Add(12, 13)),
            })
        );
    }
}
