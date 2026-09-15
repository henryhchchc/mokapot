//! Resolves formed JVM blocks into scalar SSA blocks.
use std::collections::BTreeMap;

use super::model;
use crate::ir::{
    BlockId, TryMapValues,
    generator::{
        block_formation,
        error::Error,
        identity::SsaValueId,
        ssa::{
            merge::MergePlan,
            model::{Phi, Successor},
            simplify::SimplifiedPhis,
        },
    },
};

type PhiCandidate = (SsaValueId, Vec<(BlockId, SsaValueId)>);

pub(super) fn finalize(
    blocks: Vec<block_formation::Block>,
    merge_plan: &MergePlan,
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
    let mut phis_by_block = BTreeMap::<BlockId, Vec<PhiCandidate>>::new();
    for (value, inputs) in simplified.candidates {
        let block = merge_plan
            .block_for(value)
            .ok_or(Error::MalformedControlFlow)?;
        phis_by_block
            .entry(block)
            .or_default()
            .push((value, inputs));
    }
    let mut finalized = Vec::with_capacity(blocks.len());
    for block in blocks {
        let id = block.id;
        finalized.push(finalize_block(
            block,
            phis_by_block.remove(&id).unwrap_or_default(),
            merge_plan,
            &canonical,
        )?);
    }
    if phis_by_block.is_empty() {
        Ok(finalized)
    } else {
        Err(Error::MalformedControlFlow)
    }
}
fn finalize_block(
    block: block_formation::Block,
    phis: Vec<PhiCandidate>,
    merge_plan: &MergePlan,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> Result<model::Block, Error> {
    let block_formation::Block {
        id,
        entry_frame: _,
        operations,
        terminator,
        terminator_source,
        arms,
        caught_exception,
    } = block;
    let resolve = |operand| merge_plan.resolve(operand).map(canonical);
    let caught_exception = caught_exception.map(canonical);
    let phis = phis
        .into_iter()
        .map(|(value, inputs)| Phi {
            value: canonical(value),
            inputs: inputs
                .into_iter()
                .map(|(predecessor, value)| (predecessor, canonical(value)))
                .collect(),
        })
        .collect();
    let operations = operations
        .into_iter()
        .map(|(source, operation)| {
            operation
                .try_map_values(&resolve)
                .map(|operation| (source, operation))
        })
        .collect::<Result<_, _>>()?;
    let terminator = terminator.try_map_values(&resolve)?;
    let successors = arms
        .into_iter()
        .map(
            |block_formation::Arm {
                 target,
                 transfer,
                 frame: _,
             }| {
                let transfer = transfer.try_map_values(&resolve)?;
                Ok(Successor { target, transfer })
            },
        )
        .collect::<Result<_, Error>>()?;
    Ok(model::Block {
        id,
        caught_exception,
        phis,
        operations,
        terminator,
        terminator_source,
        successors,
    })
}
