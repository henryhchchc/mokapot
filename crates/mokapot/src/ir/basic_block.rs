use super::{Operation, Successor, Terminator, ValueId};

/// A scalar value defined on entry to a basic block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockParameter {
    /// The value defined by this parameter.
    pub value: ValueId,
}

/// A maximal reachable basic block in completed `MokaIR`.
///
/// Its parameters are bound simultaneously on entry, its operations execute in
/// order, and its single terminator defines every outgoing control-flow arm
/// and may define a result available only after successful completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock<Op = Operation> {
    /// The semantic role of this block.
    pub kind: BlockKind,
    /// The scalar parameters bound at block entry.
    pub parameters: Vec<BlockParameter>,
    /// The ordinary operations in execution order.
    pub operations: Vec<Op>,
    /// The block terminator.
    pub terminator: Terminator<Successor, Op>,
}

impl<Op> BasicBlock<Op> {
    /// Maps the block's ordinary and terminator operations.
    pub(crate) fn map_operations<MappedOp>(
        self,
        mut map: impl FnMut(Op) -> MappedOp,
    ) -> BasicBlock<MappedOp> {
        BasicBlock {
            kind: self.kind,
            parameters: self.parameters,
            operations: self.operations.into_iter().map(&mut map).collect(),
            terminator: self.terminator.map_operation(map),
        }
    }
}

/// The semantic role of a basic block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockKind {
    /// An ordinary code block.
    Code,
    /// An exception-handler landing pad defining the caught exception.
    LandingPad {
        /// The caught exception available on entry.
        exception: ValueId,
    },
}
