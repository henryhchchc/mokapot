use std::{collections::HashSet, fmt};

use super::{InstructionId, ValueId, expression::Expression};

/// The kind of an ordinary Moka IR operation.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum OperationKind {
    /// Evaluates an expression and defines its result.
    #[display("{value} = {expr}")]
    Definition {
        /// The value defined by the expression.
        value: ValueId,
        /// The expression producing the value.
        expr: Expression,
    },
    /// Evaluates an expression solely for its effects.
    #[display("{expr}")]
    Effect {
        /// The effectful expression.
        expr: Expression,
    },
}

impl OperationKind {
    /// Returns the value defined by this operation, if any.
    #[must_use]
    pub const fn def(&self) -> Option<ValueId> {
        match self {
            Self::Definition { value, .. } => Some(*value),
            Self::Effect { .. } => None,
        }
    }

    /// Returns the values used by this operation.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::Definition { expr, .. } | Self::Effect { expr } => expr.uses(),
        }
    }
}

/// An identified ordinary non-phi, non-terminator operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    id: InstructionId,
    kind: OperationKind,
}

impl Operation {
    pub(crate) const fn new(id: InstructionId, kind: OperationKind) -> Self {
        Self { id, kind }
    }
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
