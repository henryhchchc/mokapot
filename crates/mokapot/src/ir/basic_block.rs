use super::{Operation, Terminator, ValueId};

/// A scalar value defined on entry to a basic block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockParameter {
    /// The value defined by this parameter.
    pub value: ValueId,
}

/// A maximal reachable basic block in completed `MokaIR`.
///
/// Its parameters are bound simultaneously on entry, its operations execute in
/// order, and its single terminator defines every outgoing control-flow arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    /// The caught exception introduced at this synthetic handler entry.
    pub caught_exception: Option<ValueId>,
    /// The scalar parameters bound at block entry.
    pub parameters: Vec<BlockParameter>,
    /// The ordinary operations in execution order.
    pub operations: Vec<Operation>,
    /// The block terminator.
    pub terminator: Terminator,
}
