//! Simplifies scalar phis and materializes them in semantic SSA blocks.

mod finalization;
mod model;
mod simplify;

use crate::ir::{
    BlockId,
    generator::{bytecode_analysis::scalar::ScalarGraph, error::Error, identity::SsaValueId},
};
pub(in crate::ir::generator) use model::Block;
use simplify::simplify_phis;

/// Scalar blocks consumed by final identity allocation and emission.
pub(in crate::ir::generator) struct SsaGraph {
    pub(in crate::ir::generator) entry: BlockId,
    pub(in crate::ir::generator) blocks: Vec<Block>,
    pub(in crate::ir::generator) this_value: Option<SsaValueId>,
    pub(in crate::ir::generator) parameter_values: Vec<SsaValueId>,
}

/// Simplifies and materializes scalar phis into final SSA blocks.
pub(super) fn construct(graph: ScalarGraph) -> Result<SsaGraph, Error> {
    let ScalarGraph {
        entry,
        blocks,
        phi_candidates,
        this_value,
        parameter_values,
    } = graph;
    let simplified = simplify_phis(phi_candidates)
        .map_err(|_| Error::internal("reachable phi definitions form a closed cycle"))?;
    let blocks = finalization::finalize(blocks, simplified)?;
    Ok(SsaGraph {
        entry,
        blocks,
        this_value,
        parameter_values,
    })
}
