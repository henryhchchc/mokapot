//! Canonicalizes the provisional SSA produced by dataflow analysis.

mod finalization;
mod simplify;

use std::collections::HashMap;

use simplify::{ParameterCandidate, simplify_parameters};

use crate::ir::{ValueId, generator::draft::DraftMethod};

/// Simplifies provisional block parameters and rewrites the draft to canonical SSA.
pub(super) fn canonicalize(draft: &mut DraftMethod) {
    let mut inputs = HashMap::<ValueId, Vec<ValueId>>::new();
    // `resolve_blocks` lowers one argument per parameter position, so the arities match.
    let entry = &draft.blocks[&draft.entry.target];
    entry
        .parameters
        .iter()
        .zip(&draft.entry.arguments)
        .for_each(|(param, &arg)| inputs.entry(param.value).or_default().push(arg));

    draft
        .blocks
        .values()
        .flat_map(|bb| bb.terminator.arms())
        .filter_map(|it| it.block_target().map(|target_bb| (it, target_bb)))
        .flat_map(|(edge, target_bb)| {
            let target = draft
                .blocks
                .get(&target_bb)
                .expect("a draft edge target belongs to the method");
            target.parameters.iter().zip(edge.arguments())
        })
        .for_each(|(param, &arg)| inputs.entry(param.value).or_default().push(arg));

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
        .collect::<HashMap<ValueId, _>>();
    let simplified = simplify_parameters(candidates);
    finalization::finalize(draft, &simplified);
}
