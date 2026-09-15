//! Collects, simplifies, and materializes scalar SSA in semantic blocks.

mod finalization;
mod merge;
mod model;
mod simplify;

use crate::ir::{
    BlockId,
    generator::{block_formation, error::Error, identity::SsaValueId},
};
use merge::{MergeCatalog, collect_phi_candidates};
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
pub(super) fn construct(graph: block_formation::BlockGraph) -> Result<Graph, Error> {
    let block_formation::BlockGraph {
        entry,
        blocks,
        merges,
        this_value,
        parameter_values,
    } = graph;
    let merge_catalog = MergeCatalog::new(merges, blocks.iter().map(|block| block.id))?;
    let candidates = collect_phi_candidates(&blocks, &merge_catalog)?;
    let simplified = simplify_phis(candidates).map_err(|_| Error::MalformedControlFlow)?;
    let blocks = finalization::finalize(blocks, &merge_catalog, simplified)?;
    Ok(Graph {
        entry,
        blocks,
        this_value,
        parameter_values,
    })
}
