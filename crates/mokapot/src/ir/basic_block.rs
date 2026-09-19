use super::{Operation, Phi, Terminator, ValueId};

/// A maximal reachable basic block in completed `MokaIR`.
///
/// Its phis are evaluated simultaneously on entry, its operations execute in
/// order, and its single terminator defines every outgoing control-flow arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    /// The caught exception introduced at this synthetic handler entry.
    pub caught_exception: Option<ValueId>,
    /// The phi nodes evaluated at block entry.
    pub phis: Vec<Phi>,
    /// The ordinary operations in execution order.
    pub operations: Vec<Operation>,
    /// The block terminator.
    pub terminator: Terminator,
}
