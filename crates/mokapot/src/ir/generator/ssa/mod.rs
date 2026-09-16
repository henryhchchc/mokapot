//! Collects, simplifies, and materializes scalar SSA in semantic blocks.

mod finalization;
mod model;
mod simplify;

use crate::ir::{
    BlockId,
    generator::{error::Error, identity::SsaValueId},
};
pub(crate) use model::Block;
pub(crate) use model::{PhiCandidate, ScalarBlock, Successor};
use simplify::simplify_phis;

/// Scalar blocks consumed by final identity allocation and emission.
pub(super) struct Graph {
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Frame-free scalar blocks and provisional phis produced by bytecode analysis.
pub(super) struct UnfinalizedGraph {
    pub entry: BlockId,
    pub blocks: Vec<ScalarBlock>,
    pub phi_candidates: std::collections::BTreeMap<SsaValueId, PhiCandidate>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Lowers frame operands, simplifies scalar phis, and finalizes SSA blocks.
pub(super) fn construct(graph: UnfinalizedGraph) -> Result<Graph, Error> {
    let UnfinalizedGraph {
        entry,
        blocks,
        phi_candidates,
        this_value,
        parameter_values,
    } = graph;
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
