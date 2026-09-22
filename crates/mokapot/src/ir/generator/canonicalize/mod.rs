//! Canonicalizes the provisional SSA produced by dataflow analysis.

mod finalization;
mod simplify;

use std::collections::HashMap;

use simplify::{ParameterCandidate, simplify_parameters};

use crate::ir::{BasicBlock, BlockId, MethodEntry, ValueId};

/// Simplifies provisional block parameters and rewrites `entry` and `blocks` to canonical SSA.
pub(super) fn canonicalize(
    entry: MethodEntry,
    blocks: HashMap<BlockId, BasicBlock>,
) -> (MethodEntry, HashMap<BlockId, BasicBlock>) {
    let mut inputs = HashMap::<ValueId, Vec<ValueId>>::new();
    // `resolve_blocks` lowers one argument per parameter position, so the arities match.
    let entry_bb = &blocks[&entry.target];
    entry_bb
        .parameters
        .iter()
        .zip(&entry.arguments)
        .for_each(|(param, &arg)| inputs.entry(param.value).or_default().push(arg));

    blocks
        .values()
        .flat_map(|bb| bb.terminator.arms())
        .filter_map(|it| it.block_target().map(|target_bb| (it, target_bb)))
        .flat_map(|(edge, target)| blocks[&target].parameters.iter().zip(edge.arguments()))
        .for_each(|(param, &arg)| inputs.entry(param.value).or_default().push(arg));

    let candidates = blocks
        .values()
        .flat_map(|block| &block.parameters)
        .map(|parameter| {
            let candidate = ParameterCandidate {
                inputs: inputs.remove(&parameter.value).unwrap_or_default(),
            };
            (parameter.value, candidate)
        })
        .collect::<HashMap<ValueId, _>>();
    let simplified = simplify_parameters(candidates);
    finalization::finalize(entry, blocks, &simplified)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        BlockKind, BlockParameter, NumericalId, Operation, Successor, Terminator,
        control_flow::ControlTransfer, expression::MathOperation,
    };

    fn v(id: u32) -> ValueId {
        ValueId::from_raw(id)
    }

    fn b(id: u32) -> BlockId {
        BlockId::from_raw(id)
    }

    fn arm(target: BlockId, arguments: &[ValueId]) -> Successor {
        let arguments = arguments.to_vec();
        Successor::Block {
            target,
            arguments,
            transfer: ControlTransfer::Unconditional,
        }
    }

    fn goto(target: BlockId, arguments: &[ValueId]) -> Terminator {
        Terminator::Goto {
            target: arm(target, arguments),
        }
    }

    fn br(taken: Successor, otherwise: Successor) -> Terminator {
        Terminator::Branch { taken, otherwise }
    }

    fn ret(value: ValueId) -> Terminator {
        Terminator::Return { value: Some(value) }
    }

    fn bb(params: &[ValueId], ops: &[Operation], term: Terminator) -> BasicBlock {
        let parameters = params
            .iter()
            .map(|&value| BlockParameter { value })
            .collect();
        let operations = ops.to_vec();
        BasicBlock {
            kind: BlockKind::Code,
            parameters,
            operations,
            terminator: term,
        }
    }

    fn inc(defines: ValueId, uses: ValueId) -> Operation {
        Operation::Definition {
            value: defines,
            expr: MathOperation::Increment(uses, 1).into(),
        }
    }

    fn method_entry(target: BlockId, arguments: Vec<ValueId>) -> MethodEntry {
        MethodEntry { target, arguments }
    }

    #[test]
    fn substitutes_a_chain_of_single_input_parameters() {
        let (x, p, q, r) = (v(0), v(1), v(2), v(3));
        let (b0, b1, b2) = (b(0), b(1), b(2));
        let blocks = HashMap::from([
            (b0, bb(&[], &[], goto(b1, &[x]))),
            (b1, bb(&[p], &[inc(r, p)], goto(b2, &[p]))),
            (b2, bb(&[q], &[], ret(q))),
        ]);
        // Every single-input parameter is replaced by the value it forwards.
        let (_, blocks) = canonicalize(method_entry(b0, vec![]), blocks);
        assert_eq!(blocks[&b0], bb(&[], &[], goto(b1, &[])));
        assert_eq!(blocks[&b1], bb(&[], &[inc(r, x)], goto(b2, &[])));
        assert_eq!(blocks[&b2], bb(&[], &[], ret(x)));
    }

    #[test]
    fn retains_a_genuine_join_parameter_and_its_parallel_arguments() {
        let (left, right, join) = (v(0), v(1), v(2));
        let (b0, b1) = (b(0), b(1));
        // Two distinct inputs make the join a real choice, so both edges keep their argument.
        let branch = br(arm(b1, &[left]), arm(b1, &[right]));
        let blocks = HashMap::from([
            (b0, bb(&[], &[], branch.clone())),
            (b1, bb(&[join], &[], ret(join))),
        ]);
        let (_, blocks) = canonicalize(method_entry(b0, vec![]), blocks);
        assert_eq!(blocks[&b0], bb(&[], &[], branch));
        assert_eq!(blocks[&b1], bb(&[join], &[], ret(join)));
    }

    #[test]
    fn drops_eliminated_argument_slots_from_edges_and_entry_arguments() {
        let (kept, dead, kept_arg, dead_arg, back_edge, result) =
            (v(0), v(1), v(2), v(3), v(4), v(5));
        let (b0, b1) = (b(0), b(1));
        let blocks = HashMap::from([
            (b0, bb(&[kept, dead], &[inc(result, kept)], ret(dead))),
            // A back edge making the entry block's parameters a join of two inputs.
            (b1, bb(&[], &[], goto(b0, &[back_edge, dead_arg]))),
        ]);
        // `kept` joins two distinct inputs while `dead` forwards one, so only it is replaced.
        let expected = bb(&[kept], &[inc(result, kept)], ret(dead_arg));
        let entry = method_entry(b0, vec![kept_arg, dead_arg]);
        let (entry, blocks) = canonicalize(entry, blocks);
        assert_eq!(entry, method_entry(b0, vec![kept_arg]));
        assert_eq!(blocks[&b0], expected);
        assert_eq!(blocks[&b1], bb(&[], &[], goto(b0, &[back_edge])));
    }
}
