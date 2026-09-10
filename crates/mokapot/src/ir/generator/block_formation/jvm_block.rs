use super::super::{
    BlockId, ControlTransfer, Instruction, JvmStackFrame, Location, OperandState, SsaValueId,
};

/// One exact outgoing edge from a formed JVM block.
#[derive(Debug)]
pub(in crate::ir::generator) struct JvmBlockArm {
    pub target: BlockId,
    pub transfer: ControlTransfer<OperandState>,
    pub frame: JvmStackFrame,
}

/// A maximal JVM block with exact symbolic operands and outgoing frames.
#[derive(Debug)]
pub(in crate::ir::generator) struct JvmBlock {
    pub id: BlockId,
    pub entry_frame: JvmStackFrame,
    pub instructions: Vec<(Location, Instruction)>,
    pub arms: Vec<JvmBlockArm>,
    pub caught_exception: Option<SsaValueId>,
}
