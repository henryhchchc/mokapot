use super::{BlockId, Operation, Phi, Terminator};

/// A maximal reachable basic block in completed `MokaIR`.
///
/// Its phis are evaluated simultaneously on entry, its operations execute in
/// order, and its single terminator defines every outgoing control-flow arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    pub(super) id: BlockId,
    pub(super) phis: Vec<Phi>,
    pub(super) operations: Vec<Operation>,
    pub(super) terminator: Terminator,
}

impl BasicBlock {
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
