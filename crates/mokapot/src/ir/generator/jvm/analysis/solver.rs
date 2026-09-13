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
    jvm::{frame::JvmStackFrame, normalization::Location},
};

struct SolverState {
    entry: (Location, JvmStackFrame<OperandState>),
    completed: BTreeMap<Location, AnalyzedLocation>,
    predecessors: BTreeMap<Location, BTreeSet<Location>>,
    dirty: BTreeSet<Location>,
    queued_inputs: BTreeMap<Location, JvmStackFrame<OperandState>>,
}

#[cfg(test)]
type CompletedFingerprint = BTreeMap<
    Location,
    (
        JvmStackFrame<OperandState>,
        Vec<(Location, JvmStackFrame<OperandState>)>,
    ),
>;

#[cfg(test)]
#[derive(Clone, PartialEq, Eq, Hash)]
struct SolverFingerprint {
    completed: CompletedFingerprint,
    dirty: BTreeSet<Location>,
    queued_inputs: BTreeMap<Location, JvmStackFrame<OperandState>>,
}

impl SolverState {
    fn new(entry_location: Location, initial_frame: JvmStackFrame<OperandState>) -> Self {
        Self {
            entry: (entry_location, initial_frame),
            completed: BTreeMap::new(),
            predecessors: BTreeMap::new(),
            dirty: BTreeSet::from([entry_location]),
            queued_inputs: BTreeMap::new(),
        }
    }

    fn recompute_dirty(&mut self) {
        while let Some(location) = self.dirty.pop_first() {
            match self.recompute_frame(location) {
                Some(incoming)
                    if self.completed.get(&location).map(|result| &result.incoming)
                        != Some(&incoming) =>
                {
                    self.queued_inputs.insert(location, incoming);
                }
                None => self.remove_unreachable(location),
                Some(_) => {
                    self.queued_inputs.remove(&location);
                }
            }
        }
    }

    fn recompute_frame(&self, location: Location) -> Option<JvmStackFrame<OperandState>> {
        let seed = (location == self.entry.0).then_some(&self.entry.1);
        let mut incoming = seed.into_iter().chain(
            self.predecessors
                .get(&location)
                .into_iter()
                .flatten()
                .flat_map(|source| {
                    self.completed
                        .get(source)
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
        self.queued_inputs.remove(&location);
        let Some(result) = self.completed.remove(&location) else {
            return;
        };
        for target in result
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

    fn replace_result(&mut self, location: Location, result: AnalyzedLocation) {
        let current_targets = targets(&result.outgoing);
        let previous_targets = self
            .completed
            .get(&location)
            .map_or_else(BTreeSet::new, |previous| targets(&previous.outgoing));
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
        self.dirty
            .extend(previous_targets.union(&current_targets).copied());
        self.completed.insert(location, result);
        if targets_removed {
            self.purge_detached_nodes();
        }
    }

    fn purge_detached_nodes(&mut self) {
        let reachable = self.reachable_locations();
        self.completed
            .retain(|location, _| reachable.contains(location));
        self.queued_inputs
            .retain(|location, _| reachable.contains(location));
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
        let mut pending = BTreeSet::from([self.entry.0]);
        while let Some(location) = pending.pop_first() {
            if !reachable.insert(location) {
                continue;
            }
            if let Some(result) = self.completed.get(&location) {
                pending.extend(result.outgoing.iter().map(|edge| edge.target));
            }
        }
        reachable
    }

    #[cfg(test)]
    fn fingerprint(&self) -> SolverFingerprint {
        SolverFingerprint {
            completed: self
                .completed
                .iter()
                .map(|(&location, result)| {
                    let outgoing = result
                        .outgoing
                        .iter()
                        .map(|edge| (edge.target, edge.frame.clone()))
                        .collect();
                    (location, (result.incoming.clone(), outgoing))
                })
                .collect(),
            dirty: self.dirty.clone(),
            queued_inputs: self.queued_inputs.clone(),
        }
    }
}

fn targets(outgoing: &[JvmOutgoing]) -> BTreeSet<Location> {
    outgoing.iter().map(|edge| edge.target).collect()
}

pub(super) fn solve(
    analyzer: &mut JvmFrameAnalyzer<'_>,
    entry_location: Location,
    initial_frame: JvmStackFrame<OperandState>,
) -> Result<BTreeMap<Location, AnalyzedLocation>, MokaIRBuildError> {
    let mut state = SolverState::new(entry_location, initial_frame);

    #[cfg(test)]
    let mut seen = HashSet::new();

    // Normalization bounds the set of locations, while definition and merge
    // identities are interned by location. This bounds the symbolic frames the
    // solver can encounter, but replacement is not monotone; the test-only
    // fingerprint catches a repeated state if transfer changes ever introduce
    // an oscillation.
    while !state.dirty.is_empty() || !state.queued_inputs.is_empty() {
        #[cfg(test)]
        assert!(
            seen.insert(state.fingerprint()),
            "JVM frame analysis entered a solver-state cycle"
        );
        state.recompute_dirty();
        let Some((location, incoming)) = state.queued_inputs.pop_first() else {
            continue;
        };
        let result = analyzer.transfer(location, incoming)?;
        state.replace_result(location, result);
    }

    Ok(state.completed)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        control_flow::ControlTransfer,
        generator::{
            identity::SsaValueId,
            jvm::{
                frame::{Entry, FrameSlot},
                instruction::Instruction,
            },
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

    fn result_to(
        target: Location,
        outgoing_frame: JvmStackFrame<OperandState>,
    ) -> AnalyzedLocation {
        AnalyzedLocation {
            incoming: frame(0),
            instruction: Instruction::Erased,
            outgoing: vec![JvmOutgoing {
                target,
                transfer: ControlTransfer::Unconditional,
                frame: outgoing_frame,
            }],
            caught_exception: None,
        }
    }

    #[test]
    fn parallel_edges_from_one_predecessor_all_contribute() {
        let source = Location::entry(0.into());
        let target = Location::entry(1.into());
        let mut state = SolverState::new(source, frame(0));
        state.recompute_dirty();

        state.replace_result(
            source,
            AnalyzedLocation {
                incoming: frame(0),
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
                caught_exception: None,
            },
        );
        state.recompute_dirty();

        assert_eq!(state.predecessors[&target], BTreeSet::from([source]));
        assert_eq!(
            state.queued_inputs[&target].local_variables(),
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

        state.replace_result(source, result_to(target, frame(1)));
        state.recompute_dirty();
        assert_eq!(
            state.queued_inputs[&target].local_variables(),
            &[Entry::Value(OperandState::Value(SsaValueId::new(1)))]
        );

        state.replace_result(source, result_to(target, frame(2)));
        state.recompute_dirty();
        assert_eq!(
            state.queued_inputs[&target].local_variables(),
            &[Entry::Value(OperandState::Value(SsaValueId::new(2)))]
        );

        state.replace_result(
            source,
            AnalyzedLocation {
                incoming: frame(0),
                instruction: Instruction::Erased,
                outgoing: Vec::new(),
                caught_exception: None,
            },
        );
        state.recompute_dirty();
        assert!(!state.completed.contains_key(&target));
        assert!(!state.queued_inputs.contains_key(&target));
    }

    #[test]
    fn removing_the_last_seed_edge_purges_a_detached_cycle() {
        let source = Location::entry(0.into());
        let first = Location::entry(1.into());
        let second = Location::entry(2.into());
        let mut state = SolverState::new(source, frame(0));
        state.recompute_dirty();

        state.replace_result(source, result_to(first, frame(1)));
        state.recompute_dirty();
        state.replace_result(first, result_to(second, frame(1)));
        state.recompute_dirty();
        state.replace_result(second, result_to(first, frame(1)));
        state.recompute_dirty();
        assert!(state.completed.contains_key(&first));
        assert!(state.completed.contains_key(&second));

        state.replace_result(
            source,
            AnalyzedLocation {
                incoming: frame(0),
                instruction: Instruction::Erased,
                outgoing: Vec::new(),
                caught_exception: None,
            },
        );

        assert_eq!(
            state.completed.keys().copied().collect::<Vec<_>>(),
            [source]
        );
        assert!(state.predecessors.is_empty());
        assert!(!state.queued_inputs.contains_key(&first));
        assert!(!state.queued_inputs.contains_key(&second));
    }
}
