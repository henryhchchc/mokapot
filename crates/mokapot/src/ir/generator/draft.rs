//! Mutable internal IR shared by analysis, canonicalization, and finishing.

use std::collections::BTreeMap;

use crate::{
    ir::{BlockId, EdgeId, OperationKind, TerminatorKind, ValueId, control_flow::ControlTransfer},
    jvm::code::ProgramCounter,
};

/// A method under construction.
pub(super) struct DraftMethod {
    pub entry: BlockId,
    pub blocks: BTreeMap<BlockId, DraftBlock>,
    pub this_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}

/// A block under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftBlock {
    pub caught_exception: Option<ValueId>,
    pub phis: Vec<DraftPhi>,
    pub operations: Vec<DraftOperation>,
    pub terminator: DraftTerminator,
}

/// A predecessor-indexed phi under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftPhi {
    pub value: ValueId,
    pub inputs: Vec<(BlockId, ValueId)>,
}

/// An operation and its bytecode origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftOperation {
    pub kind: OperationKind,
    pub origin: Option<ProgramCounter>,
}

/// A terminator, its successors, and its bytecode origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftTerminator {
    pub kind: TerminatorKind,
    pub successors: Vec<DraftSuccessor>,
    pub origin: Option<ProgramCounter>,
}

/// A normalized successor under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DraftSuccessor {
    pub id: EdgeId,
    pub target: BlockId,
    pub transfer: ControlTransfer,
}
