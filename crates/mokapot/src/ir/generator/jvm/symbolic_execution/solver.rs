//! Fixed-point execution of symbolic JVM frames.
//!
//! Each source node's retained edges are the authoritative frame contributions.
//! Replacing them updates a predecessor index before destination frames are
//! recomputed, so obsolete symbolic values never leak into the result.

use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
use std::collections::HashSet;

use super::{
    executor::Executor,
    fact::{Edge, FrameMergeSite, Node, Value},
};
use crate::ir::generator::{
    error::MokaIRBuildError,
    jvm::{NodeAddress, frame::Frame},
};

struct State {
    entry_input: (NodeAddress, Frame<Value>),
    nodes: BTreeMap<NodeAddress, Node>,
    predecessors: BTreeMap<NodeAddress, BTreeSet<NodeAddress>>,
    inputs_to_recompute: BTreeSet<NodeAddress>,
    pending_executions: BTreeMap<NodeAddress, Frame<Value>>,
}

#[cfg(test)]
type NodeFingerprint = BTreeMap<NodeAddress, (Frame<Value>, Vec<(NodeAddress, Frame<Value>)>)>;

#[cfg(test)]
#[derive(Clone, PartialEq, Eq, Hash)]
struct Fingerprint {
    nodes: NodeFingerprint,
    inputs_to_recompute: BTreeSet<NodeAddress>,
    pending_executions: BTreeMap<NodeAddress, Frame<Value>>,
}

impl State {
    fn new(entry_location: NodeAddress, initial_frame: Frame<Value>) -> Self {
        Self {
            entry_input: (entry_location, initial_frame),
            nodes: BTreeMap::new(),
            predecessors: BTreeMap::new(),
            inputs_to_recompute: BTreeSet::from([entry_location]),
            pending_executions: BTreeMap::new(),
        }
    }

    fn recompute_inputs(&mut self) {
        while let Some(location) = self.inputs_to_recompute.pop_first() {
            match self.recompute_input_frame(location) {
                Some(incoming_frame)
                    if self.nodes.get(&location).map(|node| &node.incoming_frame)
                        != Some(&incoming_frame) =>
                {
                    self.pending_executions.insert(location, incoming_frame);
                }
                None => self.remove_unreachable_node(location),
                Some(_) => {
                    self.pending_executions.remove(&location);
                }
            }
        }
    }

    fn recompute_input_frame(&self, location: NodeAddress) -> Option<Frame<Value>> {
        let entry_frame = (location == self.entry_input.0).then_some(&self.entry_input.1);
        let mut contributions = entry_frame.into_iter().chain(
            self.predecessors
                .get(&location)
                .into_iter()
                .flatten()
                .flat_map(|source| {
                    self.nodes
                        .get(source)
                        .expect("predecessors must have retained nodes")
                        .outgoing_edges
                        .iter()
                        .filter(move |edge| edge.target == location)
                        .map(|edge| &edge.target_frame)
                }),
        );
        contributions.next().cloned().map(|mut frame| {
            for contribution in contributions {
                merge_input_frame_at(location, &mut frame, contribution.clone());
            }
            frame
        })
    }

    fn remove_unreachable_node(&mut self, location: NodeAddress) {
        self.pending_executions.remove(&location);
        let Some(node) = self.nodes.remove(&location) else {
            return;
        };
        for target in node
            .outgoing_edges
            .into_iter()
            .map(|outgoing| outgoing.target)
            .collect::<BTreeSet<_>>()
        {
            let predecessors = self
                .predecessors
                .get_mut(&target)
                .expect("an outgoing target has a predecessor");
            predecessors.remove(&location);
            if predecessors.is_empty() {
                self.predecessors.remove(&target);
            }
            self.inputs_to_recompute.insert(target);
        }
        self.remove_unreachable_nodes();
    }

    fn replace_node(&mut self, location: NodeAddress, node: Node) {
        let current_targets = edge_targets(&node.outgoing_edges);
        let previous_targets = self
            .nodes
            .get(&location)
            .map_or_else(BTreeSet::new, |previous| {
                edge_targets(&previous.outgoing_edges)
            });
        let targets_removed = !previous_targets.is_subset(&current_targets);
        for target in previous_targets.difference(&current_targets) {
            let predecessors = self
                .predecessors
                .get_mut(target)
                .expect("an outgoing target has a predecessor");
            predecessors.remove(&location);
            if predecessors.is_empty() {
                self.predecessors.remove(target);
            }
        }
        for &target in current_targets.difference(&previous_targets) {
            self.predecessors
                .entry(target)
                .or_default()
                .insert(location);
        }
        self.inputs_to_recompute
            .extend(previous_targets.union(&current_targets).copied());
        self.nodes.insert(location, node);
        if targets_removed {
            self.remove_unreachable_nodes();
        }
    }

    fn remove_unreachable_nodes(&mut self) {
        let reachable = self.reachable_locations();
        self.nodes
            .retain(|location, _| reachable.contains(location));
        self.pending_executions
            .retain(|location, _| reachable.contains(location));
        self.inputs_to_recompute
            .retain(|location| reachable.contains(location));

        let mut changed = BTreeSet::new();
        self.predecessors.retain(|target, sources| {
            if !reachable.contains(target) {
                return false;
            }
            let previous_len = sources.len();
            sources.retain(|source| reachable.contains(source));
            if sources.len() != previous_len {
                changed.insert(*target);
            }
            !sources.is_empty()
        });
        self.inputs_to_recompute.extend(changed);
    }

    fn reachable_locations(&self) -> BTreeSet<NodeAddress> {
        let mut reachable = BTreeSet::new();
        let mut pending = BTreeSet::from([self.entry_input.0]);
        while let Some(location) = pending.pop_first() {
            if !reachable.insert(location) {
                continue;
            }
            if let Some(node) = self.nodes.get(&location) {
                pending.extend(node.outgoing_edges.iter().map(|edge| edge.target));
            }
        }
        reachable
    }

    #[cfg(test)]
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint {
            nodes: self
                .nodes
                .iter()
                .map(|(&location, node)| {
                    let outgoing = node
                        .outgoing_edges
                        .iter()
                        .map(|edge| (edge.target, edge.target_frame.clone()))
                        .collect();
                    (location, (node.incoming_frame.clone(), outgoing))
                })
                .collect(),
            inputs_to_recompute: self.inputs_to_recompute.clone(),
            pending_executions: self.pending_executions.clone(),
        }
    }
}

fn edge_targets(outgoing: &[Edge]) -> BTreeSet<NodeAddress> {
    outgoing.iter().map(|edge| edge.target).collect()
}

pub(super) fn execute_to_fixpoint(
    executor: &mut Executor<'_>,
    entry_location: NodeAddress,
    initial_frame: Frame<Value>,
) -> Result<BTreeMap<NodeAddress, Node>, MokaIRBuildError> {
    let mut state = State::new(entry_location, initial_frame);

    #[cfg(test)]
    let mut seen = HashSet::new();

    // Subroutine expansion bounds the set of locations, while definition and
    // merge identities are interned by location. This bounds the symbolic frames
    // the solver can encounter, but replacement is not monotone; the test-only
    // fingerprint catches a repeated state if execution changes ever introduce
    // an oscillation.
    while !state.inputs_to_recompute.is_empty() || !state.pending_executions.is_empty() {
        #[cfg(test)]
        assert!(
            seen.insert(state.fingerprint()),
            "JVM symbolic execution entered a solver-state cycle"
        );
        state.recompute_inputs();
        let Some((location, incoming_frame)) = state.pending_executions.pop_first() else {
            continue;
        };
        let node = executor.execute_location(location, incoming_frame)?;
        state.replace_node(location, node);
    }

    Ok(state.nodes)
}

pub(super) fn merge_input_frame_at(
    location: NodeAddress,
    frame: &mut Frame<Value>,
    contribution: Frame<Value>,
) -> bool {
    frame.merge_from_with(contribution, |slot, lhs, rhs| {
        if *lhs == rhs {
            return false;
        }
        let identity = FrameMergeSite { location, slot };
        let merged = match (*lhs, rhs) {
            (Value::Invalid | Value::ReturnAddress(_), _)
            | (_, Value::Invalid | Value::ReturnAddress(_)) => Value::Invalid,
            (Value::Merged(current), _) if current == identity => return false,
            _ => Value::Merged(identity),
        };
        if *lhs == merged {
            false
        } else {
            *lhs = merged;
            true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ir::{
            control_flow::ControlTransfer,
            generator::{
                identity::SsaValueId,
                jvm::{
                    NodeAddress,
                    frame::{Entry, Position},
                    instruction::RegisterInstruction,
                    symbolic_execution::executor::Executor,
                },
                tests::method,
            },
        },
        jvm::code::Instruction as JvmInstruction,
    };

    fn frame(value: u32) -> Frame<Value> {
        Frame::for_method_entry(
            &"(I)V".parse().expect("valid descriptor"),
            1,
            0,
            None,
            &[Value::Ssa(SsaValueId::new(value))],
        )
        .expect("frame fits descriptor")
    }

    fn result_to(target: NodeAddress, outgoing_frame: Frame<Value>) -> Node {
        Node {
            incoming_frame: frame(0),
            instruction: RegisterInstruction::Erased,
            outgoing_edges: vec![Edge {
                target,
                transfer: ControlTransfer::Unconditional,
                target_frame: outgoing_frame,
            }],
            caught_exception_value: None,
        }
    }

    #[test]
    fn merge_identity_is_stable_for_a_location_and_slot() {
        let location = NodeAddress::entry(0.into());
        let mut merged = frame(1);

        assert!(merge_input_frame_at(location, &mut merged, frame(2)));
        let expected = Value::Merged(FrameMergeSite {
            location,
            slot: Position::Local(0),
        });
        assert_eq!(merged.local_slots(), &[Entry::Value(expected)]);
        assert!(!merge_input_frame_at(location, &mut merged, frame(3)));
        assert_eq!(merged.local_slots(), &[Entry::Value(expected)]);
    }

    #[test]
    fn frame_merge_is_permutation_independent() {
        let location = NodeAddress::entry(0.into());
        let expected = [Entry::Value(Value::Merged(FrameMergeSite {
            location,
            slot: Position::Local(0),
        }))];

        for order in [
            [1, 2, 3],
            [1, 3, 2],
            [2, 1, 3],
            [2, 3, 1],
            [3, 1, 2],
            [3, 2, 1],
        ] {
            let mut merged = frame(order[0]);
            merge_input_frame_at(location, &mut merged, frame(order[1]));
            merge_input_frame_at(location, &mut merged, frame(order[2]));
            assert_eq!(merged.local_slots(), expected);
        }
    }

    #[test]
    fn reprocessing_replaces_stale_predecessor_output() {
        let method = method(
            [
                (0, JvmInstruction::IConst0),
                (1, JvmInstruction::Nop),
                (2, JvmInstruction::Pop),
                (3, JvmInstruction::IConst1),
                (4, JvmInstruction::Goto(1.into())),
            ],
            "()V",
            vec![],
        );
        let mut executor = Executor::for_method(&method).expect("valid method");
        let nodes = executor.execute_reachable_locations().expect("valid loop");

        assert_eq!(
            nodes[&NodeAddress::entry(2.into())]
                .incoming_frame
                .operand_slots(),
            &[Entry::Value(Value::Merged(FrameMergeSite {
                location: NodeAddress::entry(1.into()),
                slot: Position::Stack(0),
            }))]
        );
    }

    #[test]
    fn parallel_edges_from_one_predecessor_all_contribute() {
        let source = NodeAddress::entry(0.into());
        let target = NodeAddress::entry(1.into());
        let mut state = State::new(source, frame(0));
        state.recompute_inputs();

        state.replace_node(
            source,
            Node {
                incoming_frame: frame(0),
                instruction: RegisterInstruction::Erased,
                outgoing_edges: vec![
                    Edge {
                        target,
                        transfer: ControlTransfer::Unconditional,
                        target_frame: frame(1),
                    },
                    Edge {
                        target,
                        transfer: ControlTransfer::Unconditional,
                        target_frame: frame(2),
                    },
                ],
                caught_exception_value: None,
            },
        );
        state.recompute_inputs();

        assert_eq!(state.predecessors[&target], BTreeSet::from([source]));
        assert_eq!(
            state.pending_executions[&target].local_slots(),
            &[Entry::Value(Value::Merged(FrameMergeSite {
                location: target,
                slot: Position::Local(0),
            }))]
        );
    }

    #[test]
    fn replacing_an_edge_discards_its_superseded_frame() {
        let source = NodeAddress::entry(0.into());
        let target = NodeAddress::entry(1.into());
        let mut state = State::new(source, frame(0));
        state.recompute_inputs();

        state.replace_node(source, result_to(target, frame(1)));
        state.recompute_inputs();
        assert_eq!(
            state.pending_executions[&target].local_slots(),
            &[Entry::Value(Value::Ssa(SsaValueId::new(1)))]
        );

        state.replace_node(source, result_to(target, frame(2)));
        state.recompute_inputs();
        assert_eq!(
            state.pending_executions[&target].local_slots(),
            &[Entry::Value(Value::Ssa(SsaValueId::new(2)))]
        );

        state.replace_node(
            source,
            Node {
                incoming_frame: frame(0),
                instruction: RegisterInstruction::Erased,
                outgoing_edges: Vec::new(),
                caught_exception_value: None,
            },
        );
        state.recompute_inputs();
        assert!(!state.nodes.contains_key(&target));
        assert!(!state.pending_executions.contains_key(&target));
    }

    #[test]
    fn removing_the_last_seed_edge_purges_a_detached_cycle() {
        let source = NodeAddress::entry(0.into());
        let first = NodeAddress::entry(1.into());
        let second = NodeAddress::entry(2.into());
        let mut state = State::new(source, frame(0));
        state.recompute_inputs();

        state.replace_node(source, result_to(first, frame(1)));
        state.recompute_inputs();
        state.replace_node(first, result_to(second, frame(1)));
        state.recompute_inputs();
        state.replace_node(second, result_to(first, frame(1)));
        state.recompute_inputs();
        assert!(state.nodes.contains_key(&first));
        assert!(state.nodes.contains_key(&second));

        state.replace_node(
            source,
            Node {
                incoming_frame: frame(0),
                instruction: RegisterInstruction::Erased,
                outgoing_edges: Vec::new(),
                caught_exception_value: None,
            },
        );

        assert_eq!(state.nodes.keys().copied().collect::<Vec<_>>(), [source]);
        assert!(state.predecessors.is_empty());
        assert!(!state.pending_executions.contains_key(&first));
        assert!(!state.pending_executions.contains_key(&second));
    }
}
