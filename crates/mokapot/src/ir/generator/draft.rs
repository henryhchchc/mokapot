//! Mutable internal IR shared by analysis, canonicalization, and finishing.

use std::collections::BTreeMap;

use crate::{
    ir::{
        BlockId, BlockKind, EdgeId, OperationKind, SuccessorTarget, Terminator, ValueId,
        control_flow::ControlTransfer,
    },
    jvm::code::ProgramCounter,
};

/// A method under construction.
pub(super) struct DraftMethod {
    pub entry: BlockId,
    pub entry_arguments: Vec<ValueId>,
    pub blocks: BTreeMap<BlockId, DraftBlock>,
    pub this_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}

/// A block under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftBlock {
    pub kind: BlockKind,
    pub parameters: Vec<DraftParameter>,
    pub operations: Vec<DraftOperation>,
    pub terminator: DraftTerminator,
}

/// A block parameter under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftParameter {
    pub value: ValueId,
}

/// An operation and its bytecode origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftOperation {
    pub kind: OperationKind,
    pub origin: Option<ProgramCounter>,
}

/// A complete terminator and its bytecode origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftTerminator {
    pub shape: DraftTerminatorShape,
    pub origin: Option<ProgramCounter>,
}

/// The structural shape of a terminator under construction.
pub(super) type DraftTerminatorShape = Terminator<DraftEdge>;

/// A normalized successor under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftEdge {
    pub id: EdgeId,
    pub target: SuccessorTarget,
    pub arguments: Vec<ValueId>,
    pub transfer: ControlTransfer,
}
