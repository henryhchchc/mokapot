//! Constructs and simplifies scalar SSA from planned basic blocks.

mod finalization;
mod merge;
mod model;
mod simplify;

use super::block_formation::{BlockEntry, JvmBlockGraph};
use super::{
    BTreeMap, BlockId, ControlTransfer, Instruction, MergeIdentity, MokaIRBuildError, OperandState,
    OperationKind, SsaValueId, TerminatorKind,
};
use merge::{PhiPlan, collect_phi_candidates};
pub(in crate::ir::generator) use model::SsaBlock;
use model::{SsaPhi, SsaSuccessor};
use simplify::simplify_phis;

/// Fully lowered scalar SSA consumed by final identity allocation and emission.
pub(super) struct SsaGraph {
    pub entry: BlockId,
    pub blocks: Vec<SsaBlock>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Assembles and simplifies phis, then discards JVM frame state.
pub(super) fn construct(graph: JvmBlockGraph) -> Result<SsaGraph, MokaIRBuildError> {
    let JvmBlockGraph {
        entry,
        initial_frame,
        blocks,
        phi_blocks,
        merge_values,
        this_value,
        parameter_values,
    } = graph;
    let phi_plan = PhiPlan::new(phi_blocks, merge_values);
    let preheader = match entry {
        BlockEntry::Direct(_) => None,
        BlockEntry::Preheader {
            synthetic,
            bytecode,
        } => Some((
            bytecode,
            synthetic,
            initial_frame
                .as_ref()
                .ok_or(MokaIRBuildError::MalformedControlFlow)?,
        )),
    };
    let collected = collect_phi_candidates(blocks, phi_plan, preheader)?;
    let simplified =
        simplify_phis(collected.candidates).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
    let blocks = finalization::finalize(entry, collected.blocks, &collected.phi_plan, simplified)?;
    Ok(SsaGraph {
        entry: entry.method_entry(),
        blocks,
        this_value,
        parameter_values,
    })
}
