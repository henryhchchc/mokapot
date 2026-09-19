//! Canonicalizes the provisional SSA produced by bytecode analysis.

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

/// Canonical SSA blocks with materialized phis, ready to finish.
pub(super) struct CanonicalGraph {
    pub entry: BlockId,
    pub blocks: BTreeMap<BlockId, Block>,
    pub this_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}

/// Simplifies and materializes scalar phis into final SSA blocks.
pub(super) fn canonicalize(graph: ScalarGraph) -> Result<CanonicalGraph, Error> {
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
    Ok(CanonicalGraph {
        entry,
        blocks,
        this_value,
        parameter_values,
    })
}
