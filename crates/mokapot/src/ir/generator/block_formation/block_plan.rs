use super::{BlockId, Location};

/// The analyzed locations assigned to one maximal basic block.
#[derive(Debug, Clone)]
pub(in crate::ir::generator) struct BlockPlan {
    pub(in crate::ir::generator) id: BlockId,
    pub(in crate::ir::generator) locations: Vec<Location>,
}
