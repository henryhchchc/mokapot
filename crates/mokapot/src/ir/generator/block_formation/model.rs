use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            identity::SsaValueId,
            jvm::{
                analysis::{MergeIdentity, OperandState},
                frame::JvmStackFrame,
            },
        },
    },
    jvm::code::ProgramCounter,
};
use std::collections::BTreeMap;

/// Block-level JVM graph consumed by SSA construction.
pub(crate) struct JvmBlockGraph {
    pub entry: BlockId,
    pub blocks: Vec<JvmBlock>,
    pub phi_blocks: BTreeMap<SsaValueId, BlockId>,
    pub merge_values: BTreeMap<MergeIdentity, SsaValueId>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// One exact outgoing edge from a formed JVM block.
#[derive(Debug)]
pub(crate) struct JvmBlockArm {
    pub target: BlockId,
    pub transfer: ControlTransfer<OperandState>,
    pub frame: JvmStackFrame<OperandState>,
}

/// A maximal JVM block with exact symbolic operands and outgoing frames.
#[derive(Debug)]
pub(crate) struct JvmBlock {
    pub id: BlockId,
    pub entry_frame: JvmStackFrame<OperandState>,
    pub operations: Vec<(ProgramCounter, OperationKind<OperandState>)>,
    pub terminator: TerminatorKind<OperandState>,
    pub terminator_source: Option<ProgramCounter>,
    pub arms: Vec<JvmBlockArm>,
    pub caught_exception: Option<SsaValueId>,
}
