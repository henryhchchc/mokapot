//! Canonicalizes the provisional SSA produced by dataflow analysis.

mod apply;
mod simplify;
mod tarjan;

use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use crate::ir::{BasicBlock, BlockId, MethodEntry, ValueId};

type Substitutions = HashMap<ValueId, ValueId>;

/// Simplifies provisional block parameters and rewrites `entry` and `blocks` to canonical SSA.
pub(super) fn canonicalize_values(
    entry: &mut MethodEntry,
    blocks: &mut HashMap<BlockId, BasicBlock>,
) {
    let mut arguments = {
        let entry_arguments = blocks[&entry.target]
            .parameters
            .iter()
            .zip(&entry.arguments)
            .map(|(parameter, argument)| (parameter.value, *argument));

        let edge_arguments = blocks
            .values()
            .flat_map(|bb| bb.terminator.arms())
            .filter_map(|it| it.block_target().map(|target| (it, target)))
            .flat_map(|(edge, target)| blocks[&target].parameters.iter().zip(edge.arguments()))
            .map(|(parameter, argument)| (parameter.value, *argument));
        entry_arguments.chain(edge_arguments).into_group_map()
    };
    let inputs = blocks
        .values()
        .flat_map(|it| &it.parameters)
        .map(|parameter| {
            let candidate = ParameterInputs {
                arguments: arguments
                    .remove(&parameter.value)
                    .expect("every block parameter takes an incoming argument"),
            };
            (parameter.value, candidate)
        })
        .collect();
    SimplifiedParameters::for_block_inputs(inputs).apply(entry, blocks);
}

/// The result of simplifying provisional block parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SimplifiedParameters {
    /// Canonical replacements for eliminated parameter results.
    pub(super) substitutions: HashMap<ValueId, ValueId>,
    /// Results of parameters that represent genuine choices after rewriting.
    pub(super) retained: HashSet<ValueId>,
}

/// The arguments supplied to a block parameter by all incoming edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParameterInputs {
    pub(super) arguments: Vec<ValueId>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::canonicalize_values;
    use crate::ir::{expression::MathOperation, test::prelude::*};

    #[test]
    fn substitutes_a_chain_of_single_input_parameters() {
        let [x, p, q, r] = ids(0);
        let [b0, b1, b2] = ids(0);
        let ops = [def(r, MathOperation::Increment(p, 1))];
        let canonical_ops = [def(r, MathOperation::Increment(x, 1))];

        let mut blocks = HashMap::from([
            code(b0, goto(b1, [x])),
            bb(b1, [p], &ops, goto(b2, [p])),
            bb(b2, [q], &[], ret(q)),
        ]);
        canonicalize_values(&mut method_entry(b0, []), &mut blocks);

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

        let mut canonicalized = blocks.clone();
        canonicalize_values(&mut method_entry(b0, []), &mut canonicalized);
        assert_eq!(canonicalized, blocks);
    }

    #[test]
    fn drops_eliminated_argument_slots_from_edges_and_entry_arguments() {
        let [kept, dead, kept_arg, dead_arg, back_edge, result] = ids(0);
        let [b0, b1] = ids(0);
        let ops = [def(result, MathOperation::Increment(kept, 1))];

        let mut blocks = HashMap::from([
            bb(b0, [kept, dead], &ops, ret(dead)),
            // A back edge making the entry block's parameters a join of two inputs.
            code(b1, goto(b0, [back_edge, dead_arg])),
        ]);
        let mut entry = method_entry(b0, [kept_arg, dead_arg]);
        canonicalize_values(&mut entry, &mut blocks);

        // `kept` joins two distinct inputs while `dead` forwards one, so only it is replaced.
        let expected = HashMap::from([
            bb(b0, [kept], &ops, ret(dead_arg)),
            code(b1, goto(b0, [back_edge])),
        ]);
        assert_eq!(entry, method_entry(b0, [kept_arg]));
        assert_eq!(blocks, expected);
    }

    #[test]
    #[should_panic(expected = "closed cycle")]
    fn rejects_a_closed_parameter_cycle() {
        let [a, b] = ids(0);
        let [b1, b2] = ids(0);
        let mut blocks = HashMap::from([
            bb(b1, [a], &[], goto(b2, [a])),
            bb(b2, [b], &[], goto(b1, [b])),
        ]);
        canonicalize_values(&mut method_entry(b1, []), &mut blocks);
    }
}
