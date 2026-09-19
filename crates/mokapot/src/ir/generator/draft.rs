//! Mutable internal IR shared by analysis, canonicalization, and finishing.

use std::collections::HashMap;

use crate::ir::{BasicBlock, BlockId, SourceMap, ValueId};

/// A method under construction.
pub(super) struct DraftMethod {
    pub entry: BlockId,
    pub entry_arguments: Vec<ValueId>,
    pub blocks: HashMap<BlockId, BasicBlock>,
    pub source_map: SourceMap,
    pub this_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}
