use super::super::{Location, ReturnAddress};
use super::{BlockId, ControlTransfer, Instruction, OperationKind, SsaValueId, TerminatorKind};
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

/// A JVM block after frame-dependent operands have been resolved to SSA values.
pub(super) struct LoweredBlock {
    pub id: BlockId,
    pub instructions: Vec<(Location, Instruction<LoweredOperand>)>,
    pub arms: Vec<LoweredBlockArm>,
    pub caught_exception: Option<SsaValueId>,
}

/// One outgoing arm from a frame-free lowered block.
pub(super) struct LoweredBlockArm {
    pub target: BlockId,
    pub transfer: ControlTransfer<LoweredOperand>,
}

/// An operand retained after JVM frames and merge identities have been discarded.
///
/// `ReturnAddress` preserves legacy `ret` provenance until finalization; it is
/// never a scalar SSA value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum LoweredOperand {
    Value(SsaValueId),
    ReturnAddress(ReturnAddress),
}
