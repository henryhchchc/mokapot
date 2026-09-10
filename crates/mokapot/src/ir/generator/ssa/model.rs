use std::collections::BTreeMap;

use super::{
    BlockId, JvmStackFrame, LiftedControlTransfer, LiftedInstruction, Location, MokaIRBuildError,
    SsaFrameValue, SsaValueId,
};
use crate::ir::generator::block_formation::BlockPlan;

#[derive(Debug, Clone)]
pub(in crate::ir::generator) struct SsaArm {
    pub(in crate::ir::generator) target: BlockId,
    pub(in crate::ir::generator) transfer: LiftedControlTransfer<SsaFrameValue>,
    pub(in crate::ir::generator) frame: JvmStackFrame<SsaFrameValue>,
}

#[derive(Debug, Clone)]
pub(in crate::ir::generator) struct SsaBlock {
    pub(in crate::ir::generator) plan: BlockPlan,
    pub(in crate::ir::generator) entry_frame: JvmStackFrame<SsaFrameValue>,
    pub(in crate::ir::generator) instructions: Vec<(Location, LiftedInstruction<SsaFrameValue>)>,
    pub(in crate::ir::generator) arms: Vec<SsaArm>,
}

pub(in crate::ir::generator) type SsaEntryFrames = (
    BTreeMap<BlockId, JvmStackFrame<SsaFrameValue>>,
    BTreeMap<SsaValueId, BlockId>,
);

pub(in crate::ir::generator) type PairedFrameValue = (Option<SsaValueId>, Option<SsaValueId>);

pub(in crate::ir::generator) fn next_ssa_value(
    next: &mut u32,
) -> Result<SsaValueId, MokaIRBuildError> {
    let value = SsaValueId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    Ok(value)
}
