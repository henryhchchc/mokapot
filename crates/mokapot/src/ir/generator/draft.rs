//! Mutable internal IR shared by analysis, canonicalization, and definition indexing.

use std::collections::HashMap;

use crate::ir::{BasicBlock, BlockId, ValueId};

/// A method under construction.
pub(super) struct DraftMethod {
    pub entry: BlockId,
    pub entry_arguments: Vec<ValueId>,
    pub blocks: HashMap<BlockId, BasicBlock>,
    pub this: Option<ValueId>,
    pub parameters: Vec<ValueId>,
}
