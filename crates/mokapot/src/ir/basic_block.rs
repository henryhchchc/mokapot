use super::{BlockId, Operation, Phi, Terminator};

/// A maximal basic block ending in exactly one terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    id: BlockId,
    phis: Vec<Phi>,
    operations: Vec<Operation>,
    terminator: Terminator,
}

impl BasicBlock {
    pub(crate) const fn new(
        id: BlockId,
        phis: Vec<Phi>,
        operations: Vec<Operation>,
        terminator: Terminator,
    ) -> Self {
        Self {
            id,
            phis,
            operations,
            terminator,
        }
    }
    /// Returns this block's method-local identity.
    #[must_use]
    pub const fn id(&self) -> BlockId {
        self.id
    }
    /// Returns the phi nodes evaluated at block entry.
    #[must_use]
    pub fn phis(&self) -> &[Phi] {
        &self.phis
    }
    /// Returns the ordinary operations in execution order.
    #[must_use]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }
    /// Returns the block terminator.
    #[must_use]
    pub const fn terminator(&self) -> &Terminator {
        &self.terminator
    }
}
