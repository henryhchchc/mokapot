//! The scalar-graph contract produced by bytecode analysis and consumed by SSA
//! construction.

use std::collections::BTreeMap;

use crate::ir::{
    BlockId, EdgeId, OperationKind, TerminatorKind, ValueId, control_flow::ControlTransfer,
};

/// A block whose JVM-frame operands have all been lowered to scalar values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScalarBlock {
    pub caught_exception: Option<ValueId>,
    pub operations: Vec<OperationKind>,
    pub terminator: TerminatorKind,
    pub successors: Vec<ScalarSuccessor>,
}

/// A normalized edge with its frame-dependent transfer attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScalarSuccessor {
    pub id: EdgeId,
    pub target: BlockId,
    pub transfer: ControlTransfer,
}

/// A predecessor-indexed scalar phi candidate and its placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PhiCandidate {
    pub placement: BlockId,
    pub inputs: Vec<(BlockId, ValueId)>,
}

/// Frame-free scalar blocks and provisional phis produced by bytecode analysis.
pub(crate) struct ScalarGraph {
    pub entry: BlockId,
    pub blocks: BTreeMap<BlockId, ScalarBlock>,
    pub phi_candidates: BTreeMap<ValueId, PhiCandidate>,
    pub this_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}
