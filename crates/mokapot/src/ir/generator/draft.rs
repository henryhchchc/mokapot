//! Mutable internal IR shared by analysis, canonicalization, and finishing.

use std::collections::BTreeMap;

use crate::ir::{
    BlockId, BlockKind, EdgeId, OperationKind, SourceMap, SuccessorTarget, Terminator, ValueId,
    control_flow::ControlTransfer,
};

/// A method under construction.
pub(super) struct DraftMethod {
    pub entry: BlockId,
    pub entry_arguments: Vec<ValueId>,
    pub blocks: BTreeMap<BlockId, DraftBlock>,
    pub source_map: SourceMap,
    pub this_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}

/// A block under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftBlock {
    pub kind: BlockKind,
    pub parameters: Vec<DraftParameter>,
    pub operations: Vec<OperationKind>,
    pub terminator: DraftTerminator,
}

/// A block parameter under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftParameter {
    pub value: ValueId,
}

/// The structural shape of a terminator under construction.
pub(super) type DraftTerminator = Terminator<DraftEdge, OperationKind>;

/// A normalized successor under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftEdge {
    pub id: EdgeId,
    pub target: SuccessorTarget,
    pub arguments: Vec<ValueId>,
    pub transfer: ControlTransfer,
}
