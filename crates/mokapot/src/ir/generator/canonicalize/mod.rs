//! Canonicalizes the provisional SSA produced by bytecode analysis.

mod finalization;
mod simplify;

use std::collections::BTreeMap;

use crate::ir::{
    ValueId,
    generator::{draft::DraftMethod, error::Error},
};
use simplify::simplify_phis;

/// Simplifies provisional phis and rewrites the draft to canonical SSA.
pub(super) fn canonicalize(draft: &mut DraftMethod) -> Result<(), Error> {
    let candidates = draft
        .blocks
        .values()
        .flat_map(|block| block.phis.iter().cloned())
        .map(|phi| (phi.value, phi))
        .collect::<BTreeMap<ValueId, _>>();
    let simplified = simplify_phis(candidates)
        .map_err(|_| Error::internal("reachable phi definitions form a closed cycle"))?;
    finalization::finalize(draft, simplified)
}
