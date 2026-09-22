//! Canonicalizes the provisional SSA produced by dataflow analysis.

mod finalization;
mod simplify;

use std::collections::HashMap;

use itertools::Itertools;
use simplify::{ParameterCandidate, simplify_parameters};

use crate::ir::{BasicBlock, BlockId, MethodEntry};

/// Simplifies provisional block parameters and rewrites `entry` and `blocks` to canonical SSA.
pub(super) fn canonicalize(
    entry: MethodEntry,
    blocks: HashMap<BlockId, BasicBlock>,
) -> (MethodEntry, HashMap<BlockId, BasicBlock>) {
    let mut inputs = {
        let entry_inputs = blocks[&entry.target]
            .parameters
            .iter()
            .zip(&entry.arguments)
            .map(|(p, a)| (p.value, *a));

        let block_inputs = blocks
            .values()
            .flat_map(|bb| bb.terminator.arms())
            .filter_map(|it| it.block_target().map(|target_bb| (it, target_bb)))
            .flat_map(|(edge, target)| blocks[&target].parameters.iter().zip(edge.arguments()))
            .map(|(p, a)| (p.value, *a));
        entry_inputs.chain(block_inputs).into_group_map()
    };
    let candidates = blocks
        .values()
        .flat_map(|it| &it.parameters)
        .map(|param| {
            let cand = ParameterCandidate {
                inputs: inputs
                    .remove(&param.value)
                    .expect("every block parameter takes an incoming argument"),
            };
            (param.value, cand)
        })
        .collect();
    let simplified = simplify_parameters(candidates);
    finalization::finalize(entry, blocks, &simplified)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::canonicalize;
    use crate::ir::{expression::MathOperation, test::prelude::*};

    #[test]
    fn substitutes_a_chain_of_single_input_parameters() {
        let [x, p, q, r] = ids(0);
        let [b0, b1, b2] = ids(0);
        let ops = [def(r, MathOperation::Increment(p, 1))];
        let canonical_ops = [def(r, MathOperation::Increment(x, 1))];

        let blocks = HashMap::from([
            code(b0, goto(b1, [x])),
            bb(b1, [p], &ops, goto(b2, [p])),
            bb(b2, [q], &[], ret(q)),
        ]);
        let (_, blocks) = canonicalize(method_entry(b0, []), blocks);

        // Every single-input parameter is replaced by the value it forwards.
        let expected = HashMap::from([
            code(b0, goto(b1, [])),
            bb(b1, [], &canonical_ops, goto(b2, [])),
            code(b2, ret(x)),
        ]);
        assert_eq!(blocks, expected);
    }

    #[test]
    fn retains_a_genuine_join_parameter_and_its_parallel_arguments() {
        let [left, right, join] = ids(0);
        let [b0, b1] = ids(0);
        // Two distinct inputs make the join a real choice, so both edges keep their argument.
        let blocks = HashMap::from([
            code(b0, branch(edge(b1, [left]), edge(b1, [right]))),
            bb(b1, [join], &[], ret(join)),
        ]);

        let (_, canonicalized) = canonicalize(method_entry(b0, []), blocks.clone());
        assert_eq!(canonicalized, blocks);
    }

    #[test]
    fn drops_eliminated_argument_slots_from_edges_and_entry_arguments() {
        let [kept, dead, kept_arg, dead_arg, back_edge, result] = ids(0);
        let [b0, b1] = ids(0);
        let ops = [def(result, MathOperation::Increment(kept, 1))];

        let blocks = HashMap::from([
            bb(b0, [kept, dead], &ops, ret(dead)),
            // A back edge making the entry block's parameters a join of two inputs.
            code(b1, goto(b0, [back_edge, dead_arg])),
        ]);
        let (entry, blocks) = canonicalize(method_entry(b0, [kept_arg, dead_arg]), blocks);

        // `kept` joins two distinct inputs while `dead` forwards one, so only it is replaced.
        let expected = HashMap::from([
            bb(b0, [kept], &ops, ret(dead_arg)),
            code(b1, goto(b0, [back_edge])),
        ]);
        assert_eq!(entry, method_entry(b0, [kept_arg]));
        assert_eq!(blocks, expected);
    }
}
