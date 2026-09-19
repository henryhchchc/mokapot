//! Simplifies scalar phis and materializes them in semantic SSA blocks.

mod finalization;
mod model;
mod simplify;

use std::collections::BTreeMap;

use crate::ir::{
    BlockId, ValueId,
    generator::{bytecode_analysis::ScalarGraph, error::Error},
};
pub(super) use model::Block;
use simplify::simplify_phis;

/// Scalar blocks with materialized phis, ready for emission.
pub(super) struct SsaGraph {
    pub entry: BlockId,
    pub blocks: BTreeMap<BlockId, Block>,
    pub this_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
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
