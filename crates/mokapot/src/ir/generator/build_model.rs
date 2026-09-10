use std::collections::BTreeMap;

use super::{
    BasicBlock, BlockId, JvmStackFrame, LiftedControlTransfer, LiftedInstruction, Location,
    MokaIRBuildError, ProvisionalValueId, ScalarValue, SourceMap, ValueDefinition, ValueId,
};

#[derive(Debug)]
pub(super) struct GeneratedMethod {
    pub(super) entry: BlockId,
    pub(super) blocks: Vec<BasicBlock>,
    pub(super) source_map: SourceMap,
    pub(super) this_value: Option<ValueId>,
    pub(super) parameter_values: Vec<ValueId>,
    pub(super) caught_exceptions: BTreeMap<BlockId, ValueId>,
    pub(super) value_definitions: Vec<ValueDefinition>,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedBlock {
    pub(super) id: BlockId,
    pub(super) pcs: Vec<Location>,
}

#[derive(Debug, Clone)]
pub(super) struct ScalarArm {
    pub(super) target: BlockId,
    pub(super) transfer: LiftedControlTransfer<ScalarValue>,
    pub(super) frame: JvmStackFrame<ScalarValue>,
}

#[derive(Debug, Clone)]
pub(super) struct ScalarBlock {
    pub(super) plan: PlannedBlock,
    pub(super) entry_frame: JvmStackFrame<ScalarValue>,
    pub(super) instructions: Vec<(Location, LiftedInstruction<ScalarValue>)>,
    pub(super) arms: Vec<ScalarArm>,
}

pub(super) type OutgoingState<OP> = (Location, LiftedControlTransfer<OP>, JvmStackFrame<OP>);
pub(super) type ScalarEntryFrames = (
    BTreeMap<BlockId, JvmStackFrame<ScalarValue>>,
    BTreeMap<ProvisionalValueId, BlockId>,
);
pub(super) type PairedFrameValue = (Option<ProvisionalValueId>, Option<ProvisionalValueId>);

pub(super) fn next_temp_value(next: &mut u32) -> Result<ProvisionalValueId, MokaIRBuildError> {
    let value = ProvisionalValueId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    Ok(value)
}
