//! Collects, simplifies, and materializes scalar SSA in semantic blocks.

mod finalization;
mod merge;
mod model;
mod simplify;

use super::block_formation::JvmBlockGraph;
use super::{
    BTreeMap, BlockId, ControlTransfer, MergeIdentity, MokaIRBuildError, OperandState,
    OperationKind, SsaValueId, TerminatorKind,
};
use merge::{MergePlan, collect_phi_candidates};
pub(in crate::ir::generator) use model::SsaBlock;
use model::{SsaPhi, SsaSuccessor};
use simplify::simplify_phis;

/// Scalar blocks consumed by final identity allocation and emission.
pub(super) struct SsaGraph {
    pub entry: BlockId,
    pub blocks: Vec<SsaBlock>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Collects and simplifies phis, then resolves frame operands into scalar blocks.
pub(super) fn construct(graph: JvmBlockGraph) -> Result<SsaGraph, MokaIRBuildError> {
    let JvmBlockGraph {
        entry,
        blocks,
        phi_blocks,
        merge_values,
        this_value,
        parameter_values,
    } = graph;
    let merge_plan = MergePlan::new(merge_values, phi_blocks);
    let candidates = collect_phi_candidates(&blocks, &merge_plan)?;
    let simplified =
        simplify_phis(candidates).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
    let blocks = finalization::finalize(blocks, &merge_plan, simplified)?;
    Ok(SsaGraph {
        entry,
        blocks,
        this_value,
        parameter_values,
    })
}
