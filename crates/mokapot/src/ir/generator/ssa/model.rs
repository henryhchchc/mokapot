use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind, control_flow::ControlTransfer,
        generator::identity::SsaValueId,
    },
    jvm::code::ProgramCounter,
};

/// One outgoing arm from an SSA block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct Successor {
    pub(in crate::ir::generator) target: BlockId,
    pub(in crate::ir::generator) transfer: ControlTransfer<SsaValueId>,
}

/// A block whose JVM-frame operands have all been lowered to scalar values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct ScalarBlock {
    pub(in crate::ir::generator) id: BlockId,
    pub(in crate::ir::generator) caught_exception: Option<SsaValueId>,
    pub(in crate::ir::generator) operations: Vec<(ProgramCounter, OperationKind<SsaValueId>)>,
    pub(in crate::ir::generator) terminator: TerminatorKind<SsaValueId>,
    pub(in crate::ir::generator) terminator_source: Option<ProgramCounter>,
    pub(in crate::ir::generator) successors: Vec<Successor>,
}

/// A predecessor-indexed scalar phi candidate and its placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct PhiCandidate {
    pub(in crate::ir::generator) placement: BlockId,
    pub(in crate::ir::generator) inputs: Vec<(BlockId, SsaValueId)>,
}

/// A scalar SSA block ready for final IR emission.
pub(in crate::ir::generator) struct Block {
    pub(in crate::ir::generator) id: BlockId,
    pub(in crate::ir::generator) caught_exception: Option<SsaValueId>,
    pub(in crate::ir::generator) phis: Vec<Phi>,
    pub(in crate::ir::generator) operations: Vec<(ProgramCounter, OperationKind<SsaValueId>)>,
    pub(in crate::ir::generator) terminator: TerminatorKind<SsaValueId>,
    pub(in crate::ir::generator) terminator_source: Option<ProgramCounter>,
    pub(in crate::ir::generator) successors: Vec<Successor>,
}

/// A retained SSA phi and its predecessor-indexed inputs.
pub(in crate::ir::generator) struct Phi {
    pub(in crate::ir::generator) value: SsaValueId,
    pub(in crate::ir::generator) inputs: Vec<(BlockId, SsaValueId)>,
}
