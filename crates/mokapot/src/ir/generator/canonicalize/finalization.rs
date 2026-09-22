//! Applies scalar substitutions and removes eliminated block parameters.
use std::{collections::HashMap, convert::Infallible};

use crate::ir::{
    BasicBlock, BlockId, BlockKind, MethodEntry, Successor, ValueId,
    generator::{canonicalize::simplify::SimplifiedParameters, remap::RemapValues},
};

pub(super) fn finalize(
    mut entry: MethodEntry,
    mut blocks: HashMap<BlockId, BasicBlock>,
    simplified: &SimplifiedParameters,
) -> (MethodEntry, HashMap<BlockId, BasicBlock>) {
    let canonical = |it| simplified.remaps.get(&it).copied().unwrap_or(it);
    let retained = blocks
        .iter_mut()
        .map(|(&id, block)| {
            let positions = std::mem::take(&mut block.parameters)
                .into_iter()
                .enumerate()
                .filter(|(_, parameter)| simplified.retained.contains(&parameter.value))
                .collect::<Vec<_>>();
            block.parameters = positions.iter().map(|(_, parameter)| *parameter).collect();
            (id, positions.into_iter().map(|(index, _)| index).collect())
        })
        .collect::<HashMap<BlockId, Vec<usize>>>();

    entry.arguments = retained[&entry.target]
        .iter()
        .map(|&index| canonical(entry.arguments[index]))
        .collect();

    for block in blocks.values_mut() {
        finalize_block(block, &retained, &canonical);
    }

    (entry, blocks)
}

fn finalize_block(
    block: &mut BasicBlock,
    retained: &HashMap<BlockId, Vec<usize>>,
    canonical: &impl Fn(ValueId) -> ValueId,
) {
    if let BlockKind::LandingPad { exception } = &mut block.kind {
        *exception = canonical(*exception);
    }
    for parameter in &mut block.parameters {
        parameter.value = canonical(parameter.value);
    }
    for operation in &mut block.operations {
        apply_substitutions(operation, canonical);
    }
    apply_substitutions(&mut block.terminator, canonical);
    block.terminator.arms_mut().for_each(|edge| {
        if let Successor::Block {
            target,
            arguments,
            transfer,
            ..
        } = edge
        {
            *arguments = retained[target]
                .iter()
                .map(|&index| canonical(arguments[index]))
                .collect();
            apply_substitutions(transfer, canonical);
        }
    });
}

fn apply_substitutions<T: RemapValues>(value: &mut T, canonical: &impl Fn(ValueId) -> ValueId) {
    match value.try_remap_values(&mut |value| Ok::<_, Infallible>(canonical(value))) {
        Ok(()) => {}
        Err(never) => match never {},
    }
}
