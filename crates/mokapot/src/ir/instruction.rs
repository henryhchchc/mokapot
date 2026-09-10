use std::{collections::HashSet, fmt};

use super::{InstructionId, ValueId, expression::Expression};

/// The ordinary operation performed by a Moka IR instruction.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum InstructionKind {
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

impl InstructionKind {
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

/// An identified ordinary instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    id: InstructionId,
    kind: InstructionKind,
}

impl Instruction {
    pub(crate) const fn new(id: InstructionId, kind: InstructionKind) -> Self {
        Self { id, kind }
    }
    /// Returns this instruction's method-local identity.
    #[must_use]
    pub const fn id(&self) -> InstructionId {
        self.id
    }
    /// Returns the operation performed by this instruction.
    #[must_use]
    pub const fn kind(&self) -> &InstructionKind {
        &self.kind
    }
    /// Returns the value defined by this instruction, if any.
    #[must_use]
    pub const fn def(&self) -> Option<ValueId> {
        self.kind.def()
    }
    /// Returns the values used by this instruction.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        self.kind.uses()
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}
