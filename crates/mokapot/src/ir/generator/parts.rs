//! Mutable internal IR shared by analysis, canonicalization, and definition indexing.

use std::collections::HashMap;

use crate::ir::{BasicBlock, BlockId, MethodEntry, SourceMap, ValueId};

/// A method under construction.
pub(super) struct IrParts {
    pub entry: MethodEntry,
    pub blocks: HashMap<BlockId, BasicBlock>,
    pub this: Option<ValueId>,
    pub parameters: Vec<ValueId>,
    pub source_map: SourceMap,
}
