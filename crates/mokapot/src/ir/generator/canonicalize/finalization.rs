//! Applies scalar substitutions and removes eliminated block parameters.
use std::{collections::HashMap, convert::Infallible};

use crate::ir::{
    BasicBlock, BlockId, BlockKind, MethodEntry, Successor, ValueId,
    generator::{canonicalize::simplify::SimplifiedParameters, remap::RemapValues},
};

pub(super) fn rewrite_values(
    entry: &mut MethodEntry,
    blocks: &mut HashMap<BlockId, BasicBlock>,
    simplified: &SimplifiedParameters,
) {
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
        rewrite_block(block, &retained, &canonical);
    }
}

fn rewrite_block(
    block: &mut BasicBlock,
    retained: &HashMap<BlockId, Vec<usize>>,
    canonical: &impl Fn(ValueId) -> ValueId,
) {
    let BasicBlock {
        kind,
        parameters,
        operations,
        terminator,
    } = block;
    if let BlockKind::LandingPad { exception } = kind {
        *exception = canonical(*exception);
    }
    for parameter in parameters {
        parameter.value = canonical(parameter.value);
    }
    for operation in operations {
        rewrite_ops(operation, canonical);
    }
    rewrite_ops(terminator, canonical);
    for edge in terminator.arms_mut() {
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
            rewrite_ops(transfer, canonical);
        }
    }
}

fn rewrite_ops<T: RemapValues>(value: &mut T, canonical: &impl Fn(ValueId) -> ValueId) {
    value.try_remap_values(&mut |value| Ok::<_, Infallible>(canonical(value)));
}
