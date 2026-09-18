//! Mutable internal IR shared by analysis, canonicalization, and finishing.

use std::collections::BTreeMap;

use crate::ir::{BasicBlock, BlockId, SourceMap, ValueId};

/// A method under construction.
pub(super) struct DraftMethod {
    pub entry: BlockId,
    pub entry_arguments: Vec<ValueId>,
    pub blocks: BTreeMap<BlockId, BasicBlock>,
    pub source_map: SourceMap,
    pub this_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}
