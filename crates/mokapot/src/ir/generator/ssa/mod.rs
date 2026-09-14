//! Collects, simplifies, and materializes scalar SSA in semantic blocks.

mod finalization;
mod merge;
mod model;
mod simplify;

use crate::ir::{
    BlockId,
    generator::{block_formation, error::Error, identity::SsaValueId},
};
use merge::{MergePlan, collect_phi_candidates};
pub(crate) use model::Block;
use simplify::simplify_phis;

/// Scalar blocks consumed by final identity allocation and emission.
pub(super) struct Graph {
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Collects and simplifies phis, then resolves frame operands into scalar blocks.
pub(super) fn construct(graph: block_formation::Graph) -> Result<Graph, Error> {
    let block_formation::Graph {
        entry,
        blocks,
        phi_blocks,
        merge_values,
        this_value,
        parameter_values,
    } = graph;
    let merge_plan = MergePlan::new(merge_values, phi_blocks);
    let candidates = collect_phi_candidates(&blocks, &merge_plan)?;
    let simplified = simplify_phis(candidates).map_err(|_| Error::MalformedControlFlow)?;
    let blocks = finalization::finalize(blocks, &merge_plan, simplified)?;
    Ok(Graph {
        entry,
        blocks,
        this_value,
        parameter_values,
    })
}
