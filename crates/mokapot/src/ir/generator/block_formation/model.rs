use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            identity::SsaValueId,
            jvm::{
                frame::Frame,
                symbolic_execution::{self, FrameMergeSite},
            },
        },
    },
    jvm::code::ProgramCounter,
};
use std::collections::BTreeMap;

/// Block-level JVM graph consumed by SSA construction.
pub(crate) struct Graph {
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub phi_blocks: BTreeMap<SsaValueId, BlockId>,
    pub merge_values: BTreeMap<FrameMergeSite, SsaValueId>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// One exact outgoing edge from a formed JVM block.
#[derive(Debug)]
pub(crate) struct Arm {
    pub target: BlockId,
    pub transfer: ControlTransfer<symbolic_execution::Value>,
    pub frame: Frame<symbolic_execution::Value>,
}

/// A maximal JVM block with exact symbolic operands and outgoing frames.
#[derive(Debug)]
pub(crate) struct Block {
    pub id: BlockId,
    pub entry_frame: Frame<symbolic_execution::Value>,
    pub operations: Vec<(ProgramCounter, OperationKind<symbolic_execution::Value>)>,
    pub terminator: TerminatorKind<symbolic_execution::Value>,
    pub terminator_source: Option<ProgramCounter>,
    pub arms: Vec<Arm>,
    pub caught_exception: Option<SsaValueId>,
}
