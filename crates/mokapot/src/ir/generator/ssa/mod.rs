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
use merge::collect_phi_candidates;
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
    let preheader = match entry {
        BlockEntry::Direct(_) => None,
        BlockEntry::Preheader {
            synthetic,
            bytecode,
        } => Some((bytecode, synthetic, &initial_frame)),
    };
    let candidates = collect_phi_candidates(&blocks, &phi_blocks, &merge_values, preheader)?;
    let simplified =
        simplify_phis(candidates).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
    let blocks = finalization::finalize(entry, blocks, &phi_blocks, &merge_values, simplified)?;
    Ok(SsaGraph {
        entry: entry.method_entry(),
        blocks,
        this_value,
        parameter_values,
    })
}
