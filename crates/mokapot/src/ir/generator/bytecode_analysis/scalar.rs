//! The scalar-graph contract produced by bytecode analysis and consumed by SSA
//! construction.

use crate::{
    ir::{BlockId, OperationKind, TerminatorKind, ValueId, control_flow::ControlTransfer},
    jvm::code::ProgramCounter,
};

/// One outgoing arm from an SSA block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct Successor {
    pub(in crate::ir::generator) target: BlockId,
    pub(in crate::ir::generator) transfer: ControlTransfer,
}

/// A block whose JVM-frame operands have all been lowered to scalar values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct ScalarBlock {
    pub(in crate::ir::generator) id: BlockId,
    pub(in crate::ir::generator) caught_exception: Option<ValueId>,
    pub(in crate::ir::generator) operations: Vec<(ProgramCounter, OperationKind)>,
    pub(in crate::ir::generator) terminator: TerminatorKind,
    pub(in crate::ir::generator) terminator_source: Option<ProgramCounter>,
    pub(in crate::ir::generator) successors: Vec<Successor>,
}

/// A predecessor-indexed scalar phi candidate and its placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct PhiCandidate {
    pub(in crate::ir::generator) placement: BlockId,
    pub(in crate::ir::generator) inputs: Vec<(BlockId, ValueId)>,
}

/// Frame-free scalar blocks and provisional phis produced by bytecode analysis.
pub(in crate::ir::generator) struct ScalarGraph {
    pub(in crate::ir::generator) entry: BlockId,
    pub(in crate::ir::generator) blocks: Vec<ScalarBlock>,
    pub(in crate::ir::generator) phi_candidates: std::collections::BTreeMap<ValueId, PhiCandidate>,
    pub(in crate::ir::generator) this_value: Option<ValueId>,
    pub(in crate::ir::generator) parameter_values: Vec<ValueId>,
}
