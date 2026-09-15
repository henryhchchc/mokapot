use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind, control_flow::ControlTransfer,
        generator::identity::SsaValueId,
    },
    jvm::code::ProgramCounter,
};

/// One outgoing arm from an SSA block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Successor {
    pub target: BlockId,
    pub transfer: ControlTransfer<SsaValueId>,
}

/// A block whose JVM-frame operands have all been lowered to scalar values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScalarBlock {
    pub(super) id: BlockId,
    pub(super) caught_exception: Option<SsaValueId>,
    pub(super) operations: Vec<(ProgramCounter, OperationKind<SsaValueId>)>,
    pub(super) terminator: TerminatorKind<SsaValueId>,
    pub(super) terminator_source: Option<ProgramCounter>,
    pub(super) successors: Vec<Successor>,
}

/// A predecessor-indexed scalar phi candidate and its placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhiCandidate {
    pub(super) placement: BlockId,
    pub(super) inputs: Vec<(BlockId, SsaValueId)>,
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
