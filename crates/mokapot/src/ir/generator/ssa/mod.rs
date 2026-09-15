//! Collects, simplifies, and materializes scalar SSA in semantic blocks.

mod finalization;
mod lowering;
mod model;
mod simplify;

use crate::ir::{
    BlockId,
    generator::{block_formation, error::Error, identity::SsaValueId},
};
pub(crate) use model::Block;
use simplify::simplify_phis;

/// Scalar blocks consumed by final identity allocation and emission.
pub(super) struct Graph {
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Lowers frame operands, simplifies scalar phis, and finalizes SSA blocks.
pub(super) fn construct(graph: block_formation::BlockGraph) -> Result<Graph, Error> {
    let lowering::LoweredGraph {
        entry,
        blocks,
        phi_candidates,
        this_value,
        parameter_values,
    } = lowering::lower(graph)?;
    let simplified = simplify_phis(phi_candidates)
        .map_err(|_| Error::internal("reachable phi definitions form a closed cycle"))?;
    let blocks = finalization::finalize(blocks, simplified)?;
    Ok(Graph {
        entry,
        blocks,
        this_value,
        parameter_values,
    })
}
