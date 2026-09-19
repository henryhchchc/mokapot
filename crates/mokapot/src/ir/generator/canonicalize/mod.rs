//! Canonicalizes the provisional SSA produced by bytecode analysis.

mod finalization;
mod simplify;

use std::collections::BTreeMap;

use crate::ir::{
    ValueId,
    generator::{draft::DraftMethod, error::Error},
};
use simplify::{ParameterCandidate, simplify_parameters};

/// Simplifies provisional block parameters and rewrites the draft to canonical SSA.
pub(super) fn canonicalize(draft: &mut DraftMethod) -> Result<(), Error> {
    let mut inputs = BTreeMap::<ValueId, Vec<ValueId>>::new();
    for block in draft.blocks.values() {
        for edge in &block.terminator.successors {
            let target = draft
                .blocks
                .get(&edge.target)
                .expect("a draft edge target must belong to the method");
            assert_eq!(target.parameters.len(), edge.arguments.len());
            for (parameter, &argument) in target.parameters.iter().zip(&edge.arguments) {
                inputs.entry(parameter.value).or_default().push(argument);
            }
        }
    }
    let candidates = draft
        .blocks
        .values()
        .flat_map(|block| &block.parameters)
        .map(|parameter| {
            let candidate = ParameterCandidate {
                inputs: inputs.remove(&parameter.value).unwrap_or_default(),
            };
            (parameter.value, candidate)
        })
        .collect::<BTreeMap<ValueId, _>>();
    let simplified = simplify_parameters(candidates)
        .map_err(|_| Error::internal("reachable block parameters form a closed cycle"))?;
    finalization::finalize(draft, &simplified);
    Ok(())
}
