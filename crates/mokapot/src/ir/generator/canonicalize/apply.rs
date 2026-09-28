//! Applies parameter simplification results to the method IR.

use std::{collections::HashMap, convert::Infallible};

use super::SimplifiedParameters;
use crate::ir::{
    BasicBlock, BlockId, BlockKind, MethodEntry, Successor, ValueId, generator::remap::RemapValues,
};

impl SimplifiedParameters {
    /// Rewrites `entry` and `blocks` to canonical SSA.
    pub(super) fn apply(&self, entry: &mut MethodEntry, blocks: &mut HashMap<BlockId, BasicBlock>) {
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

        entry.arguments = kept_indices[&entry.block]
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

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::SimplifiedParameters;
    use crate::ir::{expression::MathOperation, test::prelude::*};

    #[test]
    fn drops_eliminated_argument_slots_from_edges_and_entry_arguments() {
        let [kept, dead, kept_arg, dead_arg, back_edge, result] = ids(0);
        let [b0, b1] = ids(0);
        let ops = [def(result, MathOperation::Increment(kept, 1))];
        let mut blocks = HashMap::from([
            bb(b0, [kept, dead], &ops, ret(dead)),
            code(b1, goto(b0, [back_edge, dead_arg])),
        ]);
        let mut entry = method_entry(b0, [kept_arg, dead_arg]);
        let simplified = SimplifiedParameters {
            substitutions: HashMap::from([(dead, dead_arg)]),
            retained: HashSet::from([kept]),
        };

        simplified.apply(&mut entry, &mut blocks);

        let expected = HashMap::from([
            bb(b0, [kept], &ops, ret(dead_arg)),
            code(b1, goto(b0, [back_edge])),
        ]);
        assert_eq!(entry, method_entry(b0, [kept_arg]));
        assert_eq!(blocks, expected);
    }
}
