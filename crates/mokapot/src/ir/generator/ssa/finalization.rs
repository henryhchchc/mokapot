//! Applies scalar substitutions and materializes retained phi nodes.
use std::{collections::BTreeMap, convert::Infallible};

use super::model;
use crate::ir::{
    BlockId, ValueId,
    generator::{
        bytecode_analysis::{PhiCandidate, ScalarBlock},
        error::Error,
        remap::RemapValues,
        ssa::{model::Phi, simplify::SimplifiedPhis},
    },
};

pub(super) fn finalize(
    blocks: BTreeMap<BlockId, ScalarBlock>,
    simplified: SimplifiedPhis,
) -> Result<BTreeMap<BlockId, model::Block>, Error> {
    // `simplify_phis` returns substitutions whose targets are already canonical.
    let canonical = |value| {
        simplified
            .substitutions
            .get(&value)
            .copied()
            .unwrap_or(value)
    };
    let mut phis_by_block = BTreeMap::<BlockId, Vec<Phi>>::new();
    for (value, PhiCandidate { placement, inputs }) in simplified.candidates {
        phis_by_block
            .entry(placement)
            .or_default()
            .push(Phi { value, inputs });
    }
    let finalized = blocks
        .into_iter()
        .map(|(id, block)| {
            let phis = phis_by_block.remove(&id).unwrap_or_default();
            (id, finalize_block(block, phis, &canonical))
        })
        .collect();
    if phis_by_block.is_empty() {
        Ok(finalized)
    } else {
        Err(Error::internal("a retained phi targets no scalar block"))
    }
}

fn finalize_block(
    mut scalar: ScalarBlock,
    phis: Vec<Phi>,
    canonical: &impl Fn(ValueId) -> ValueId,
) -> model::Block {
    scalar.caught_exception = scalar.caught_exception.map(canonical);
    let phis = phis
        .into_iter()
        .map(|phi| Phi {
            value: canonical(phi.value),
            inputs: phi
                .inputs
                .into_iter()
                .map(|(predecessor, value)| (predecessor, canonical(value)))
                .collect(),
        })
        .collect();
    scalar.operations = scalar
        .operations
        .into_iter()
        .map(|operation| apply_substitutions(operation, canonical))
        .collect();
    scalar.terminator = apply_substitutions(scalar.terminator, canonical);
    scalar.successors = scalar
        .successors
        .into_iter()
        .map(|(target, transfer)| (target, apply_substitutions(transfer, canonical)))
        .collect();
    model::Block { phis, scalar }
}

fn apply_substitutions<T: RemapValues>(mut value: T, canonical: &impl Fn(ValueId) -> ValueId) -> T {
    match value.try_remap_values(&mut |value| Ok::<_, Infallible>(canonical(value))) {
        Ok(()) => value,
        Err(never) => match never {},
    }
}
