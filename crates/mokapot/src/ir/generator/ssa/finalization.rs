//! Applies scalar substitutions and materializes retained phi nodes.
use std::{collections::BTreeMap, convert::Infallible};

use super::model;
use crate::ir::{
    BlockId, TryMapValues,
    generator::{
        error::Error,
        identity::SsaValueId,
        ssa::{
            model::{Phi, PhiCandidate, ScalarBlock},
            simplify::SimplifiedPhis,
        },
    },
};

pub(super) fn finalize(
    blocks: Vec<ScalarBlock>,
    simplified: SimplifiedPhis,
) -> Result<Vec<model::Block>, Error> {
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
    let mut finalized = Vec::with_capacity(blocks.len());
    for block in blocks {
        let id = block.id;
        finalized.push(finalize_block(
            block,
            phis_by_block.remove(&id).unwrap_or_default(),
            &canonical,
        ));
    }
    if phis_by_block.is_empty() {
        Ok(finalized)
    } else {
        Err(Error::MalformedControlFlow)
    }
}

fn finalize_block(
    block: ScalarBlock,
    phis: Vec<Phi>,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> model::Block {
    let ScalarBlock {
        id,
        caught_exception,
        operations,
        terminator,
        terminator_source,
        successors,
    } = block;
    let caught_exception = caught_exception.map(canonical);
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
    let operations = operations
        .into_iter()
        .map(|(source, operation)| (source, apply_substitutions(operation, canonical)))
        .collect();
    let terminator = apply_substitutions(terminator, canonical);
    let successors = successors
        .into_iter()
        .map(|successor| model::Successor {
            target: successor.target,
            transfer: apply_substitutions(successor.transfer, canonical),
        })
        .collect();
    model::Block {
        id,
        caught_exception,
        phis,
        operations,
        terminator,
        terminator_source,
        successors,
    }
}

fn apply_substitutions<T>(
    value: T,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> <T as TryMapValues<SsaValueId>>::Mapped
where
    T: TryMapValues<SsaValueId, Value = SsaValueId>,
{
    match value.try_map_values(|value| Ok::<_, Infallible>(canonical(value))) {
        Ok(mapped) => mapped,
        Err(never) => match never {},
    }
}
