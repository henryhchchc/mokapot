//! Applies scalar substitutions and removes eliminated phi nodes.
use std::convert::Infallible;

use crate::ir::{
    ValueId,
    generator::{
        canonicalize::simplify::SimplifiedPhis,
        draft::{DraftBlock, DraftMethod, DraftPhi},
        error::Error,
        remap::RemapValues,
    },
};

pub(super) fn finalize(
    draft: &mut DraftMethod,
    mut simplified: SimplifiedPhis,
) -> Result<(), Error> {
    // `simplify_phis` returns substitutions whose targets are already canonical.
    let canonical = |value| {
        simplified
            .substitutions
            .get(&value)
            .copied()
            .unwrap_or(value)
    };
    for block in draft.blocks.values_mut() {
        let retained = std::mem::take(&mut block.phis)
            .into_iter()
            .filter_map(|phi| simplified.candidates.remove(&phi.value))
            .collect();
        finalize_block(block, retained, &canonical);
    }
    if !simplified.candidates.is_empty() {
        return Err(Error::internal("a retained phi targets no draft block"));
    }
    Ok(())
}

fn finalize_block(
    block: &mut DraftBlock,
    phis: Vec<DraftPhi>,
    canonical: &impl Fn(ValueId) -> ValueId,
) {
    block.caught_exception = block.caught_exception.map(canonical);
    block.phis = phis
        .into_iter()
        .map(|phi| DraftPhi {
            value: canonical(phi.value),
            inputs: phi
                .inputs
                .into_iter()
                .map(|(predecessor, value)| (predecessor, canonical(value)))
                .collect(),
        })
        .collect();
    block.phis.sort_by_key(|phi| phi.value);
    for operation in &mut block.operations {
        apply_substitutions(&mut operation.kind, canonical);
    }
    apply_substitutions(&mut block.terminator.kind, canonical);
    for successor in &mut block.terminator.successors {
        apply_substitutions(&mut successor.transfer, canonical);
    }
}

fn apply_substitutions<T: RemapValues>(value: &mut T, canonical: &impl Fn(ValueId) -> ValueId) {
    match value.try_remap_values(&mut |value| Ok::<_, Infallible>(canonical(value))) {
        Ok(()) => {}
        Err(never) => match never {},
    }
}
