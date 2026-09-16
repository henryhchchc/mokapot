//! The scalar-graph contract produced by bytecode analysis and consumed by SSA
//! construction.

use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind, control_flow::ControlTransfer,
        generator::identity::SsaValueId,
    },
    jvm::code::ProgramCounter,
};

/// One outgoing arm from an SSA block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct Successor {
    pub(in crate::ir::generator) target: BlockId,
    pub(in crate::ir::generator) transfer: ControlTransfer<SsaValueId>,
}

/// A block whose JVM-frame operands have all been lowered to scalar values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct ScalarBlock {
    pub(in crate::ir::generator) id: BlockId,
    pub(in crate::ir::generator) caught_exception: Option<SsaValueId>,
    pub(in crate::ir::generator) operations: Vec<(ProgramCounter, OperationKind<SsaValueId>)>,
    pub(in crate::ir::generator) terminator: TerminatorKind<SsaValueId>,
    pub(in crate::ir::generator) terminator_source: Option<ProgramCounter>,
    pub(in crate::ir::generator) successors: Vec<Successor>,
}

/// A predecessor-indexed scalar phi candidate and its placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct PhiCandidate {
    pub(in crate::ir::generator) placement: BlockId,
    pub(in crate::ir::generator) inputs: Vec<(BlockId, SsaValueId)>,
}

/// Frame-free scalar blocks and provisional phis produced by bytecode analysis.
pub(in crate::ir::generator) struct ScalarGraph {
    pub(in crate::ir::generator) entry: BlockId,
    pub(in crate::ir::generator) blocks: Vec<ScalarBlock>,
    pub(in crate::ir::generator) phi_candidates:
        std::collections::BTreeMap<SsaValueId, PhiCandidate>,
    pub(in crate::ir::generator) this_value: Option<SsaValueId>,
    pub(in crate::ir::generator) parameter_values: Vec<SsaValueId>,
}
