use std::collections::BTreeMap;

use super::{
    BlockId, ControlTransfer, Instruction, JvmStackFrame, Location, MokaIRBuildError,
    OperationKind, SsaFrameValue, SsaValueId, TerminatorKind,
};
use crate::jvm::code::ProgramCounter;

#[derive(Debug, Clone)]
pub(super) struct ReplayedArm {
    pub(super) target: BlockId,
    pub(super) transfer: ControlTransfer<SsaFrameValue>,
    pub(super) frame: JvmStackFrame<SsaFrameValue>,
}

#[derive(Debug, Clone)]
pub(super) struct ReplayedBlock {
    pub(super) id: BlockId,
    pub(super) entry_frame: JvmStackFrame<SsaFrameValue>,
    pub(super) instructions: Vec<(Location, Instruction<SsaFrameValue>)>,
    pub(super) arms: Vec<ReplayedArm>,
}

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

pub(super) type SsaEntryFrames = (
    BTreeMap<BlockId, JvmStackFrame<SsaFrameValue>>,
    BTreeMap<SsaValueId, BlockId>,
);

pub(super) fn next_ssa_value(next: &mut u32) -> Result<SsaValueId, MokaIRBuildError> {
    let value = SsaValueId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    Ok(value)
}
