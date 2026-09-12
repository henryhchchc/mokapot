use super::super::{
    BlockId, ControlTransfer, JvmStackFrame, OperandState, OperationKind, ProgramCounter,
    SsaValueId, TerminatorKind,
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
    pub operations: Vec<(ProgramCounter, OperationKind<OperandState>)>,
    pub terminator: TerminatorKind<OperandState>,
    pub terminator_source: Option<ProgramCounter>,
    pub arms: Vec<JvmBlockArm>,
    pub caught_exception: Option<SsaValueId>,
}
