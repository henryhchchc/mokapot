use std::collections::HashSet;

use super::{BlockId, InstructionLocation, ValueId};

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
    Instruction(InstructionLocation),
}

/// One predecessor-selected incoming value of a phi node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhiInput {
    /// The predecessor block that supplies this value.
    pub predecessor: BlockId,
    /// The value supplied by the predecessor.
    pub value: ValueId,
}

/// A scalar value merge at basic-block entry.
///
/// Inputs are indexed by predecessor block, not edge. Parallel arms from one
/// predecessor must therefore agree on the supplied value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phi {
    /// The value defined by this phi.
    pub value: ValueId,
    /// The values selected by this phi.
    pub inputs: Vec<PhiInput>,
}

impl Phi {
    /// Returns the values selected by this phi.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        self.inputs.iter().map(|it| it.value).collect()
    }
}
