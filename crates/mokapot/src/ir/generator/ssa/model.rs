use super::{BlockId, ControlTransfer, OperationKind, SsaValueId, TerminatorKind};
use crate::jvm::code::ProgramCounter;

/// One outgoing arm from an SSA block.
pub(in crate::ir::generator) struct SsaSuccessor {
    pub target: BlockId,
    pub transfer: ControlTransfer<SsaValueId>,
}

/// A scalar SSA block ready for final IR emission.
pub(in crate::ir::generator) struct SsaBlock {
    pub id: BlockId,
    pub caught_exception: Option<SsaValueId>,
    pub phis: Vec<SsaPhi>,
    pub operations: Vec<(ProgramCounter, OperationKind<SsaValueId>)>,
    pub terminator: TerminatorKind<SsaValueId>,
    pub terminator_source: Option<ProgramCounter>,
    pub successors: Vec<SsaSuccessor>,
}

/// A retained SSA phi and its predecessor-indexed inputs.
pub(in crate::ir::generator) struct SsaPhi {
    pub value: SsaValueId,
    pub inputs: Vec<(BlockId, SsaValueId)>,
}
