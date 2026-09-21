//! Canonicalizes the provisional SSA produced by dataflow analysis.

mod finalization;
mod simplify;

use std::collections::HashMap;

use simplify::{ParameterCandidate, simplify_parameters};

use crate::ir::{ValueId, generator::parts::IrParts};

/// Simplifies provisional block parameters and rewrites the parts to canonical SSA.
pub(super) fn canonicalize(parts: &mut IrParts) {
    let mut inputs = HashMap::<ValueId, Vec<ValueId>>::new();
    // `resolve_blocks` lowers one argument per parameter position, so the arities match.
    let entry = &parts.blocks[&parts.entry.target];
    entry
        .parameters
        .iter()
        .zip(&parts.entry.arguments)
        .for_each(|(param, &arg)| inputs.entry(param.value).or_default().push(arg));

    parts
        .blocks
        .values()
        .flat_map(|bb| bb.terminator.arms())
        .filter_map(|it| it.block_target().map(|target_bb| (it, target_bb)))
        .flat_map(|(edge, target_bb)| {
            let target = parts
                .blocks
                .get(&target_bb)
                .expect("a parts edge target belongs to the method");
            target.parameters.iter().zip(edge.arguments())
        })
        .for_each(|(param, &arg)| inputs.entry(param.value).or_default().push(arg));

    let candidates = parts
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
    finalization::finalize(parts, &simplified);
}
