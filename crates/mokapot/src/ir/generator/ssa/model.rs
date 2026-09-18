use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind, ValueId,
        generator::bytecode_analysis::scalar::Successor,
    },
    jvm::code::ProgramCounter,
};

/// A scalar SSA block ready for final IR emission.
pub(in crate::ir::generator) struct Block {
    pub(in crate::ir::generator) id: BlockId,
    pub(in crate::ir::generator) caught_exception: Option<ValueId>,
    pub(in crate::ir::generator) phis: Vec<Phi>,
    pub(in crate::ir::generator) operations: Vec<(ProgramCounter, OperationKind)>,
    pub(in crate::ir::generator) terminator: TerminatorKind,
    pub(in crate::ir::generator) terminator_source: Option<ProgramCounter>,
    pub(in crate::ir::generator) successors: Vec<Successor>,
}

/// A retained SSA phi and its predecessor-indexed inputs.
pub(in crate::ir::generator) struct Phi {
    pub(in crate::ir::generator) value: ValueId,
    pub(in crate::ir::generator) inputs: Vec<(BlockId, ValueId)>,
}
