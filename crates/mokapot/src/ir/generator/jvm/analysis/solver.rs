//! Recomputing worklist specialized for symbolic JVM frames.
//!
//! Each source's retained output is the authoritative edge contribution.
//! Replacing it updates a predecessor index before destination frames are
//! recomputed, so obsolete symbolic values never leak into the result.

use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
use std::collections::HashSet;

use super::{
    analyzer::JvmFrameAnalyzer,
    fact::{AnalyzedLocation, JvmOutgoing, MergeIdentity, OperandState},
};
use crate::ir::generator::{
    error::MokaIRBuildError,
    identity::SsaValueId,
    jvm::{frame::JvmStackFrame, instruction::Instruction, normalization::Location},
};

pub(super) struct LocationOutput {
    pub(super) instruction: Instruction,
    pub(super) outgoing: Vec<JvmOutgoing>,
}

struct NodeState {
    incoming: JvmStackFrame<OperandState>,
    output: Option<LocationOutput>,
}

struct SolverState {
    entry_location: Location,
    initial_frame: JvmStackFrame<OperandState>,
    nodes: BTreeMap<Location, NodeState>,
    predecessors: BTreeMap<Location, BTreeSet<Location>>,
    dirty: BTreeSet<Location>,
    worklist: BTreeSet<Location>,
}

#[cfg(test)]
type OutputFingerprint = Vec<(Location, JvmStackFrame<OperandState>)>;

#[cfg(test)]
type NodeFingerprints =
    BTreeMap<Location, (JvmStackFrame<OperandState>, Option<OutputFingerprint>)>;

#[cfg(test)]
#[derive(Clone, PartialEq, Eq, Hash)]
struct SolverFingerprint {
    nodes: NodeFingerprints,
    predecessors: BTreeMap<Location, BTreeSet<Location>>,
    dirty: BTreeSet<Location>,
    worklist: BTreeSet<Location>,
}

impl SolverState {
    fn new(entry_location: Location, initial_frame: JvmStackFrame<OperandState>) -> Self {
        Self {
            entry_location,
            initial_frame,
            nodes: BTreeMap::new(),
            predecessors: BTreeMap::new(),
            dirty: BTreeSet::from([entry_location]),
            worklist: BTreeSet::new(),
        }
    }

    fn recompute_dirty(&mut self) {
        while let Some(location) = self.dirty.pop_first() {
            match self.recompute_frame(location) {
                Some(incoming)
                    if self.nodes.get(&location).map(|node| &node.incoming) != Some(&incoming) =>
                {
                    match self.nodes.entry(location) {
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            entry.insert(NodeState {
                                incoming,
                                output: None,
                            });
                        }
                        std::collections::btree_map::Entry::Occupied(mut entry) => {
                            entry.get_mut().incoming = incoming;
                        }
                    }
                    self.worklist.insert(location);
                }
                None => self.remove_unreachable(location),
                Some(_) => {}
            }
        }
    }

    fn recompute_frame(&self, location: Location) -> Option<JvmStackFrame<OperandState>> {
        let seed = (location == self.entry_location).then_some(&self.initial_frame);
        let mut incoming = seed.into_iter().chain(
            self.predecessors
                .get(&location)
                .into_iter()
                .flatten()
                .flat_map(|source| {
                    self.nodes
                        .get(source)
                        .and_then(|node| node.output.as_ref())
                        .expect("predecessors must have a retained output")
                        .outgoing
                        .iter()
                        .filter(move |edge| edge.target == location)
                        .map(|edge| &edge.frame)
                }),
        );
        incoming.next().cloned().map(|mut frame| {
            for contribution in incoming {
                merge_frame_at(location, &mut frame, contribution.clone());
            }
            frame
        })
    }

    fn remove_unreachable(&mut self, location: Location) {
        let Some(node) = self.nodes.remove(&location) else {
            return;
        };
        self.worklist.remove(&location);
        let Some(output) = node.output else {
            return;
        };
        for target in output
            .outgoing
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
            self.dirty.insert(target);
        }
        self.purge_detached_nodes();
    }

    fn replace_output(&mut self, location: Location, output: LocationOutput) {
        let current_targets = output
            .outgoing
            .iter()
            .map(|edge| edge.target)
            .collect::<BTreeSet<_>>();
        let previous_targets = self.nodes[&location]
            .output
            .as_ref()
            .into_iter()
            .flat_map(|previous| &previous.outgoing)
            .map(|edge| edge.target)
            .collect::<BTreeSet<_>>();
        let targets_removed = !previous_targets.is_subset(&current_targets);
        let affected = previous_targets
            .union(&current_targets)
            .copied()
            .collect::<BTreeSet<_>>();
        for target in affected {
            match (
                previous_targets.contains(&target),
                current_targets.contains(&target),
            ) {
                (false, true) => {
                    self.predecessors
                        .entry(target)
                        .or_default()
                        .insert(location);
                }
                (true, false) => {
                    let predecessors = self
                        .predecessors
                        .get_mut(&target)
                        .expect("an outgoing target has a predecessor");
                    predecessors.remove(&location);
                    if predecessors.is_empty() {
                        self.predecessors.remove(&target);
                    }
                }
                (true, true) => {}
                (false, false) => unreachable!("affected targets belong to an output"),
            }
            self.dirty.insert(target);
        }
        self.nodes
            .get_mut(&location)
            .expect("processed locations must remain reachable")
            .output = Some(output);
        if targets_removed {
            self.purge_detached_nodes();
        }
    }

    fn purge_detached_nodes(&mut self) {
        let reachable = self.reachable_locations();
        self.nodes
            .retain(|location, _| reachable.contains(location));
        self.worklist
            .retain(|location| reachable.contains(location));
        self.dirty.retain(|location| reachable.contains(location));

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
        self.dirty.extend(changed);
    }

    fn reachable_locations(&self) -> BTreeSet<Location> {
        let mut reachable = BTreeSet::new();
        let mut pending = BTreeSet::from([self.entry_location]);
        while let Some(location) = pending.pop_first() {
            if !reachable.insert(location) {
                continue;
            }
            if let Some(output) = self
                .nodes
                .get(&location)
                .and_then(|node| node.output.as_ref())
            {
                pending.extend(output.outgoing.iter().map(|edge| edge.target));
            }
        }
        reachable
    }

    fn finish(
        self,
        caught_exception_ids: &BTreeMap<Location, SsaValueId>,
    ) -> Result<BTreeMap<Location, AnalyzedLocation>, MokaIRBuildError> {
        self.nodes
            .into_iter()
            .map(|(location, node)| {
                let output = node.output.ok_or(MokaIRBuildError::MalformedControlFlow)?;
                Ok((
                    location,
                    AnalyzedLocation {
                        incoming: node.incoming,
                        instruction: output.instruction,
                        outgoing: output.outgoing,
                        caught_exception: caught_exception_ids.get(&location).copied(),
                    },
                ))
            })
            .collect()
    }

    #[cfg(test)]
    fn fingerprint(&self) -> SolverFingerprint {
        SolverFingerprint {
            nodes: self
                .nodes
                .iter()
                .map(|(&location, node)| {
                    let outgoing = node.output.as_ref().map(|output| {
                        output
                            .outgoing
                            .iter()
                            .map(|edge| (edge.target, edge.frame.clone()))
                            .collect()
                    });
                    (location, (node.incoming.clone(), outgoing))
                })
                .collect(),
            predecessors: self.predecessors.clone(),
            dirty: self.dirty.clone(),
            worklist: self.worklist.clone(),
        }
    }
}

pub(super) fn solve(
    analyzer: &mut JvmFrameAnalyzer<'_>,
    entry_location: Location,
    initial_frame: JvmStackFrame<OperandState>,
) -> Result<BTreeMap<Location, AnalyzedLocation>, MokaIRBuildError> {
    let mut state = SolverState::new(entry_location, initial_frame);

    #[cfg(test)]
    let mut seen = HashSet::new();

    // Normalization bounds the location set. Every value created by transfer is
    // interned by its defining location, and merging gives each destination
    // slot one stable identity. Recomputing can therefore retract stale edges
    // without creating an unbounded stream of fresh symbolic states.
    while !state.dirty.is_empty() || !state.worklist.is_empty() {
        #[cfg(test)]
        assert!(
            seen.insert(state.fingerprint()),
            "JVM frame analysis entered a solver-state cycle"
        );
        state.recompute_dirty();
        let Some(location) = state.worklist.pop_first() else {
            continue;
        };
        let incoming = state.nodes[&location].incoming.clone();
        let output = analyzer.transfer(location, &incoming)?;
        state.replace_output(location, output);
    }

    state.finish(&analyzer.caught_exception_ids)
}

pub(super) fn merge_frame_at(
    location: Location,
    frame: &mut JvmStackFrame<OperandState>,
    contribution: JvmStackFrame<OperandState>,
) -> bool {
    frame.join_assign_values_with(contribution, |slot, lhs, rhs| {
        if *lhs == rhs {
            return false;
        }
        let identity = MergeIdentity { location, slot };
        let merged = match (*lhs, rhs) {
            (OperandState::Invalid | OperandState::ReturnAddress(_), _)
            | (_, OperandState::Invalid | OperandState::ReturnAddress(_)) => OperandState::Invalid,
            (OperandState::Merged(current), _) if current == identity => return false,
            _ => OperandState::Merged(identity),
        };
        if *lhs == merged {
            false
        } else {
            *lhs = merged;
            true
        }
    })
}

pub(super) fn normalize_frame_for(
    location: Location,
    frame: JvmStackFrame<OperandState>,
) -> JvmStackFrame<OperandState> {
    if matches!(location, Location::Unwind) {
        frame.erase_values()
    } else {
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        control_flow::ControlTransfer,
        generator::{
            identity::SsaValueId,
            jvm::frame::{Entry, FrameSlot},
        },
    };

    fn frame(value: u32) -> JvmStackFrame<OperandState> {
        JvmStackFrame::with_inputs(
            &"(I)V".parse().expect("valid descriptor"),
            1,
            0,
            None,
            &[OperandState::Value(SsaValueId::new(value))],
        )
        .expect("frame fits descriptor")
    }

    fn output_to(target: Location, frame: JvmStackFrame<OperandState>) -> LocationOutput {
        LocationOutput {
            instruction: Instruction::Erased,
            outgoing: vec![JvmOutgoing {
                target,
                transfer: ControlTransfer::Unconditional,
                frame,
            }],
        }
    }

    #[test]
    fn parallel_edges_from_one_predecessor_all_contribute() {
        let source = Location::entry(0.into());
        let target = Location::entry(1.into());
        let mut state = SolverState::new(source, frame(0));
        state.recompute_dirty();

        state.replace_output(
            source,
            LocationOutput {
                instruction: Instruction::Erased,
                outgoing: vec![
                    JvmOutgoing {
                        target,
                        transfer: ControlTransfer::Unconditional,
                        frame: frame(1),
                    },
                    JvmOutgoing {
                        target,
                        transfer: ControlTransfer::Unconditional,
                        frame: frame(2),
                    },
                ],
            },
        );
        state.recompute_dirty();

        assert_eq!(state.predecessors[&target], BTreeSet::from([source]));
        assert_eq!(
            state.nodes[&target].incoming.local_variables(),
            &[Entry::Value(OperandState::Merged(MergeIdentity {
                location: target,
                slot: FrameSlot::Local(0),
            }))]
        );
    }

    #[test]
    fn replacing_an_edge_discards_its_superseded_frame() {
        let source = Location::entry(0.into());
        let target = Location::entry(1.into());
        let mut state = SolverState::new(source, frame(0));
        state.recompute_dirty();

        state.replace_output(source, output_to(target, frame(1)));
        state.recompute_dirty();
        assert_eq!(
            state.nodes[&target].incoming.local_variables(),
            &[Entry::Value(OperandState::Value(SsaValueId::new(1)))]
        );

        state.replace_output(source, output_to(target, frame(2)));
        state.recompute_dirty();
        assert_eq!(
            state.nodes[&target].incoming.local_variables(),
            &[Entry::Value(OperandState::Value(SsaValueId::new(2)))]
        );

        state.replace_output(
            source,
            LocationOutput {
                instruction: Instruction::Erased,
                outgoing: Vec::new(),
            },
        );
        state.recompute_dirty();
        assert!(!state.nodes.contains_key(&target));
    }

    #[test]
    fn removing_the_last_seed_edge_purges_a_detached_cycle() {
        let source = Location::entry(0.into());
        let first = Location::entry(1.into());
        let second = Location::entry(2.into());
        let mut state = SolverState::new(source, frame(0));
        state.recompute_dirty();

        state.replace_output(source, output_to(first, frame(1)));
        state.recompute_dirty();
        state.replace_output(first, output_to(second, frame(1)));
        state.recompute_dirty();
        state.replace_output(second, output_to(first, frame(1)));
        state.recompute_dirty();
        assert!(state.nodes.contains_key(&first));
        assert!(state.nodes.contains_key(&second));

        state.replace_output(
            source,
            LocationOutput {
                instruction: Instruction::Erased,
                outgoing: Vec::new(),
            },
        );

        assert_eq!(state.nodes.keys().copied().collect::<Vec<_>>(), [source]);
        assert!(state.predecessors.is_empty());
        assert!(!state.worklist.contains(&first));
        assert!(!state.worklist.contains(&second));
    }
}
