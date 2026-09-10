use super::{BlockId, JvmStackFrame, Location};

/// Abstract input facts and locations assigned to one maximal basic block.
#[derive(Debug, Clone)]
pub(in crate::ir::generator) struct JvmBlock {
    pub id: BlockId,
    pub locations: Vec<Location>,
    pub analyzed_entry: JvmStackFrame,
    pub incoming_frames: Vec<JvmStackFrame>,
}
