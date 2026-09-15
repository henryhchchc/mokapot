//! Fixed-point propagation of JVM frames through the instruction graph.
//!
//! Each source node's retained edges are the authoritative frame contributions.
//! Replacing them updates a predecessor index before destination frames are
//! recomputed, so obsolete frame values never leak into the result.

use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
use std::collections::HashSet;

use super::{NodeGraphBuilder, Edge, FrameMergeSite, Node, NodeAddress, Value};
use crate::ir::generator::{bytecode_analysis::jvm::Frame, error::Error};

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
    fn new(entry_addr: NodeAddress, initial_frame: Frame<Value>) -> Self {
        Self {
            entry_input: (entry_addr, initial_frame),
            nodes: BTreeMap::new(),
            predecessors: BTreeMap::new(),
            inputs_to_recompute: BTreeSet::from([entry_addr]),
            pending_executions: BTreeMap::new(),
        }
    }

    fn recompute_inputs(&mut self) -> Result<(), Error> {
        while let Some(addr) = self.inputs_to_recompute.pop_first() {
            match self.recompute_input_frame(addr)? {
                Some(incoming_frame)
                    if self.nodes.get(&addr).map(|node| &node.incoming_frame)
                        != Some(&incoming_frame) =>
                {
                    self.pending_executions.insert(addr, incoming_frame);
                }
                None => self.remove_unreachable_node(addr),
                Some(_) => {
                    self.pending_executions.remove(&addr);
                }
            }
        }
        Ok(())
    }

    fn recompute_input_frame(&self, addr: NodeAddress) -> Result<Option<Frame<Value>>, Error> {
        let entry_frame = (addr == self.entry_input.0).then_some(&self.entry_input.1);
        let mut contributions = entry_frame.into_iter().chain(
            self.predecessors
                .get(&addr)
                .into_iter()
                .flatten()
                .flat_map(|source| {
                    self.nodes
                        .get(source)
                        .expect("predecessors must have retained nodes")
                        .outgoing_edges
                        .iter()
                        .filter(move |edge| edge.target == addr)
                        .map(|edge| &edge.target_frame)
                }),
        );
        contributions.next().cloned().map_or(Ok(None), |mut frame| {
            for contribution in contributions {
                merge_input_frame_at(addr, &mut frame, contribution.clone())?;
            }
            Ok(Some(frame))
        })
    }

    fn remove_unreachable_node(&mut self, addr: NodeAddress) {
        self.pending_executions.remove(&addr);
        let Some(node) = self.nodes.remove(&addr) else {
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
            predecessors.remove(&addr);
            if predecessors.is_empty() {
                self.predecessors.remove(&target);
            }
            self.inputs_to_recompute.insert(target);
        }
        self.remove_unreachable_nodes();
    }

    fn replace_node(&mut self, addr: NodeAddress, node: Node) {
        let current_targets = edge_targets(&node.outgoing_edges);
        let previous_targets = self
            .nodes
            .get(&addr)
            .map_or_else(BTreeSet::new, |previous| {
                edge_targets(&previous.outgoing_edges)
            });
        let targets_removed = !previous_targets.is_subset(&current_targets);
        for target in previous_targets.difference(&current_targets) {
            let predecessors = self
                .predecessors
                .get_mut(target)
                .expect("an outgoing target has a predecessor");
            predecessors.remove(&addr);
            if predecessors.is_empty() {
                self.predecessors.remove(target);
            }
        }
        for &target in current_targets.difference(&previous_targets) {
            self.predecessors.entry(target).or_default().insert(addr);
        }
        self.inputs_to_recompute
            .extend(previous_targets.union(&current_targets).copied());
        self.nodes.insert(addr, node);
        if targets_removed {
            self.remove_unreachable_nodes();
        }
    }

    fn remove_unreachable_nodes(&mut self) {
        let reachable = self.reachable_addrs();
        self.nodes.retain(|addr, _| reachable.contains(addr));
        self.pending_executions
            .retain(|addr, _| reachable.contains(addr));
        self.inputs_to_recompute
            .retain(|addr| reachable.contains(addr));

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

    fn reachable_addrs(&self) -> BTreeSet<NodeAddress> {
        let mut reachable = BTreeSet::new();
        let mut pending = BTreeSet::from([self.entry_input.0]);
        while let Some(addr) = pending.pop_first() {
            if !reachable.insert(addr) {
                continue;
            }
            if let Some(node) = self.nodes.get(&addr) {
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
                .map(|(&addr, node)| {
                    let outgoing = node
                        .outgoing_edges
                        .iter()
                        .map(|edge| (edge.target, edge.target_frame.clone()))
                        .collect();
                    (addr, (node.incoming_frame.clone(), outgoing))
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

pub(super) fn build_to_fixpoint(
    builder: &mut NodeGraphBuilder<'_>,
    entry_addr: NodeAddress,
    initial_frame: Frame<Value>,
) -> Result<BTreeMap<NodeAddress, Node>, Error> {
    let mut state = State::new(entry_addr, initial_frame);

    #[cfg(test)]
    let mut seen = HashSet::new();

    // Subroutine expansion bounds the set of locations, while definition and
    // merge identities are interned by location. This bounds the abstract frames
    // the solver can encounter, but replacement is not monotone; the test-only
    // fingerprint catches a repeated state if execution changes ever introduce
    // an oscillation.
    while !state.inputs_to_recompute.is_empty() || !state.pending_executions.is_empty() {
        #[cfg(test)]
        assert!(
            seen.insert(state.fingerprint()),
            "JVM instruction-graph construction entered a solver-state cycle"
        );
        state.recompute_inputs()?;
        let Some((addr, incoming_frame)) = state.pending_executions.pop_first() else {
            continue;
        };
        let node = builder.build_node(addr, incoming_frame)?;
        state.replace_node(addr, node);
    }

    Ok(state.nodes)
}

pub(super) fn merge_input_frame_at(
    addr: NodeAddress,
    frame: &mut Frame<Value>,
    contribution: Frame<Value>,
) -> Result<bool, Error> {
    frame
        .merge_from_with(contribution, |slot, lhs, rhs| {
            if *lhs == rhs {
                return false;
            }
            let identity = FrameMergeSite { addr, slot };
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
        .map_err(Error::FrameMergeError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ir::{
            control_flow::ControlTransfer,
            generator::{
                bytecode_analysis::{
                    RegisterInstruction,
                    jvm::{Position, ValueCategory::Category1},
                },
                identity::SsaValueId,
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
            can_throw_synchronously: false,
            outgoing_edges: vec![Edge {
                target,
                transfer: ControlTransfer::Unconditional,
                target_frame: outgoing_frame,
            }],
            caught_exception_value: None,
        }
    }

    fn first_local(frame: &Frame<Value>) -> Value {
        *frame.locals.get(0, Category1).expect("local exists")
    }

    #[test]
    fn merge_identity_is_stable_for_a_location_and_slot() {
        let addr = NodeAddress::entry(0.into());
        let mut merged = frame(1);

        assert!(merge_input_frame_at(addr, &mut merged, frame(2)).expect("compatible frames"));
        let expected = Value::Merged(FrameMergeSite {
            addr,
            slot: Position::Local(0),
        });
        assert_eq!(first_local(&merged), expected);
        assert!(!merge_input_frame_at(addr, &mut merged, frame(3)).expect("compatible frames"));
        assert_eq!(first_local(&merged), expected);
    }

    #[test]
    fn frame_merge_is_permutation_independent() {
        let addr = NodeAddress::entry(0.into());
        let expected = Value::Merged(FrameMergeSite {
            addr,
            slot: Position::Local(0),
        });

        for order in [
            [1, 2, 3],
            [1, 3, 2],
            [2, 1, 3],
            [2, 3, 1],
            [3, 1, 2],
            [3, 2, 1],
        ] {
            let mut merged = frame(order[0]);
            merge_input_frame_at(addr, &mut merged, frame(order[1])).expect("compatible frames");
            merge_input_frame_at(addr, &mut merged, frame(order[2])).expect("compatible frames");
            assert_eq!(first_local(&merged), expected);
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
        let mut builder = NodeGraphBuilder::for_method(&method).expect("valid method");
        let nodes = builder.build_reachable_nodes().expect("valid loop");

        let mut incoming_frame = nodes[&NodeAddress::entry(2.into())].incoming_frame.clone();
        assert_eq!(
            incoming_frame
                .stack
                .pop(Category1)
                .expect("stack value exists"),
            Value::Merged(FrameMergeSite {
                addr: NodeAddress::entry(1.into()),
                slot: Position::Stack(0),
            })
        );
    }

    #[test]
    fn parallel_edges_from_one_predecessor_all_contribute() {
        let source = NodeAddress::entry(0.into());
        let target = NodeAddress::entry(1.into());
        let mut state = State::new(source, frame(0));
        state.recompute_inputs().expect("compatible frames");

        state.replace_node(
            source,
            Node {
                incoming_frame: frame(0),
                instruction: RegisterInstruction::Erased,
                can_throw_synchronously: false,
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
        state.recompute_inputs().expect("compatible frames");

        assert_eq!(state.predecessors[&target], BTreeSet::from([source]));
        assert_eq!(
            first_local(&state.pending_executions[&target]),
            Value::Merged(FrameMergeSite {
                addr: target,
                slot: Position::Local(0),
            })
        );
    }

    #[test]
    fn replacing_an_edge_discards_its_superseded_frame() {
        let source = NodeAddress::entry(0.into());
        let target = NodeAddress::entry(1.into());
        let mut state = State::new(source, frame(0));
        state.recompute_inputs().expect("compatible frames");

        state.replace_node(source, result_to(target, frame(1)));
        state.recompute_inputs().expect("compatible frames");
        assert_eq!(
            first_local(&state.pending_executions[&target]),
            Value::Ssa(SsaValueId::new(1))
        );

        state.replace_node(source, result_to(target, frame(2)));
        state.recompute_inputs().expect("compatible frames");
        assert_eq!(
            first_local(&state.pending_executions[&target]),
            Value::Ssa(SsaValueId::new(2))
        );

        state.replace_node(
            source,
            Node {
                incoming_frame: frame(0),
                instruction: RegisterInstruction::Erased,
                can_throw_synchronously: false,
                outgoing_edges: Vec::new(),
                caught_exception_value: None,
            },
        );
        state.recompute_inputs().expect("compatible frames");
        assert!(!state.nodes.contains_key(&target));
        assert!(!state.pending_executions.contains_key(&target));
    }

    #[test]
    fn removing_the_last_seed_edge_purges_a_detached_cycle() {
        let source = NodeAddress::entry(0.into());
        let first = NodeAddress::entry(1.into());
        let second = NodeAddress::entry(2.into());
        let mut state = State::new(source, frame(0));
        state.recompute_inputs().expect("compatible frames");

        state.replace_node(source, result_to(first, frame(1)));
        state.recompute_inputs().expect("compatible frames");
        state.replace_node(first, result_to(second, frame(1)));
        state.recompute_inputs().expect("compatible frames");
        state.replace_node(second, result_to(first, frame(1)));
        state.recompute_inputs().expect("compatible frames");
        assert!(state.nodes.contains_key(&first));
        assert!(state.nodes.contains_key(&second));

        state.replace_node(
            source,
            Node {
                incoming_frame: frame(0),
                instruction: RegisterInstruction::Erased,
                can_throw_synchronously: false,
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
