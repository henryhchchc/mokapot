use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind, control_flow::ControlTransfer,
        generator::identity::SsaValueId,
    },
    jvm::code::ProgramCounter,
};

/// One outgoing arm from an SSA block.
pub(crate) struct Successor {
    pub target: BlockId,
    pub transfer: ControlTransfer<SsaValueId>,
}

/// A scalar SSA block ready for final IR emission.
pub(crate) struct Block {
    pub id: BlockId,
    pub caught_exception: Option<SsaValueId>,
    pub phis: Vec<Phi>,
    pub operations: Vec<(ProgramCounter, OperationKind<SsaValueId>)>,
    pub terminator: TerminatorKind<SsaValueId>,
    pub terminator_source: Option<ProgramCounter>,
    pub successors: Vec<Successor>,
}

/// A retained SSA phi and its predecessor-indexed inputs.
pub(crate) struct Phi {
    pub value: SsaValueId,
    pub inputs: Vec<(BlockId, SsaValueId)>,
}
