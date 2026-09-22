//! Applies parameter simplification results to the method IR.

use std::{collections::HashMap, convert::Infallible};

use super::SimplifiedParameters;
use crate::ir::{
    BasicBlock, BlockId, BlockKind, MethodEntry, Successor, ValueId, generator::remap::RemapValues,
};

impl SimplifiedParameters {
    /// Rewrites `entry` and `blocks` to canonical SSA.
    pub(super) fn apply(&self, entry: &mut MethodEntry, blocks: &mut HashMap<BlockId, BasicBlock>) {
        // let canonical = |it| self.substitutions.get(&it).copied().unwrap_or(it);
        let kept_indices = blocks
            .iter_mut()
            .map(|(&id, block)| {
                let positions = std::mem::take(&mut block.parameters)
                    .into_iter()
                    .enumerate()
                    .filter(|(_, parameter)| self.retained.contains(&parameter.value))
                    .collect::<Vec<_>>();
                block.parameters = positions.iter().map(|(_, parameter)| *parameter).collect();
                (id, positions.into_iter().map(|(index, _)| index).collect())
            })
            .collect::<HashMap<BlockId, Vec<usize>>>();

        entry.arguments = kept_indices[&entry.target]
            .iter()
            .map(|&index| self.lookup(entry.arguments[index]))
            .collect();

        for block in blocks.values_mut() {
            rewrite_block(block, &kept_indices, &|it| self.lookup(it));
        }
    }

    fn lookup(&self, value: ValueId) -> ValueId {
        self.substitutions.get(&value).copied().unwrap_or(value)
    }
}

fn rewrite_block(
    block: &mut BasicBlock,
    kept_indices: &HashMap<BlockId, Vec<usize>>,
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
            *arguments = kept_indices[target]
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
