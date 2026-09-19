use crate::ir::{BlockId, OperationKind, TerminatorKind, ValueId, control_flow::ControlTransfer};

/// A scalar SSA block ready for final IR emission.
pub(crate) struct Block {
    pub caught_exception: Option<ValueId>,
    pub phis: Vec<Phi>,
    pub operations: Vec<OperationKind>,
    pub terminator: TerminatorKind,
    pub successors: Vec<(BlockId, ControlTransfer)>,
}

/// A retained SSA phi and its predecessor-indexed inputs.
pub(crate) struct Phi {
    pub value: ValueId,
    pub inputs: Vec<(BlockId, ValueId)>,
}
