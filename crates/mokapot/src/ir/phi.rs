use std::collections::HashSet;

use super::{BlockId, InstructionId, ValueId};

/// Describes where a scalar value is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueDefinition {
    /// The receiver of an instance method.
    This,
    /// A method parameter at the given parameter index.
    Parameter(u16),
    /// The exception introduced at a synthetic handler-entry block.
    CaughtException(BlockId),
    /// A value produced by an ordinary instruction or phi.
    Instruction(InstructionId),
}

/// One incoming value of a phi node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhiInput {
    predecessor: BlockId,
    value: ValueId,
}

impl PhiInput {
    pub(crate) const fn new(predecessor: BlockId, value: ValueId) -> Self {
        Self { predecessor, value }
    }

    /// Returns the predecessor selecting this input.
    #[must_use]
    pub const fn predecessor(&self) -> BlockId {
        self.predecessor
    }

    /// Returns the value supplied by the predecessor.
    #[must_use]
    pub const fn value(&self) -> ValueId {
        self.value
    }
}

/// A value merge at basic-block entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phi {
    id: InstructionId,
    value: ValueId,
    inputs: Vec<PhiInput>,
}

impl Phi {
    pub(crate) const fn new(id: InstructionId, value: ValueId, inputs: Vec<PhiInput>) -> Self {
        Self { id, value, inputs }
    }

    /// Returns this phi's method-local instruction identity.
    #[must_use]
    pub const fn id(&self) -> InstructionId {
        self.id
    }

    /// Returns the value defined by this phi.
    #[must_use]
    pub const fn value(&self) -> ValueId {
        self.value
    }

    /// Returns the predecessor-indexed inputs.
    #[must_use]
    pub fn inputs(&self) -> &[PhiInput] {
        &self.inputs
    }

    /// Returns the values selected by this phi.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        self.inputs.iter().map(PhiInput::value).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_exposes_predecessor_inputs() {
        let input = PhiInput::new(BlockId::new(1), ValueId::new(2));
        let phi = Phi::new(InstructionId::new(3), ValueId::new(4), vec![input]);
        assert_eq!(phi.id(), InstructionId::new(3));
        assert_eq!(phi.value(), ValueId::new(4));
        assert_eq!(phi.inputs(), &[input]);
    }
}
