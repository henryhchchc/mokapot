//! Abstractly executes JVM frames to determine reachable states.

pub(in crate::ir::generator) mod operand_state;

use super::{
    BTreeMap, BTreeSet, ControlTransfer, DataflowProblem, Instruction, JvmStackFrame, Location,
    Method, MethodBody, MokaIRBuildError, Normalizer, OperandState, ProgramCounter, ReturnAddress,
    SsaValueId, method,
};
use crate::{
    analysis::fixed_point::DataflowOutput,
    ir::generator::lifting::{
        fallibility::FallibilityContext,
        lift_instruction,
        semantics::{JvmSemantics, outgoing_from},
    },
};
pub(in crate::ir::generator) use operand_state::MergeIdentity;

/// Mutable state used only while solving JVM frame facts.
pub(in crate::ir::generator) struct JvmFrameAnalyzer<'method> {
    body: &'method MethodBody,
    fallibility: FallibilityContext,
    normalizer: Normalizer,
    definition_ids: BTreeMap<Location, SsaValueId>,
    caught_exception_ids: BTreeMap<Location, SsaValueId>,
    next_value_index: u32,
    this_value: Option<SsaValueId>,
    parameter_values: Vec<SsaValueId>,
    entry: Option<(Location, JvmFrameFact)>,
}

/// Transfer output retained for one reachable JVM location.
pub(in crate::ir::generator) struct JvmFlowOutput {
    instruction: Instruction,
    is_explicit_transfer: bool,
    outgoing: Vec<JvmFlowOutgoing>,
}

struct JvmFlowOutgoing {
    target: Location,
    transfer: ControlTransfer<OperandState>,
    frame: JvmFrameFact,
}

impl DataflowOutput<Location, JvmFrameFact> for JvmFlowOutput {
    fn successors<'a>(&'a self) -> impl Iterator<Item = (&'a Location, &'a JvmFrameFact)>
    where
        Location: 'a,
        JvmFrameFact: 'a,
    {
        self.outgoing
            .iter()
            .map(|outgoing| (&outgoing.target, &outgoing.frame))
    }

    fn into_successors(self) -> impl Iterator<Item = (Location, JvmFrameFact)> {
        self.outgoing
            .into_iter()
            .map(|outgoing| (outgoing.target, outgoing.frame))
    }
}

/// One outgoing edge and its exact symbolic frame.
pub(in crate::ir::generator) struct JvmOutgoing {
    pub target: Location,
    pub transfer: ControlTransfer<OperandState>,
    pub frame: JvmStackFrame,
}

/// A frame tagged with the location at which its values are merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct JvmFrameFact {
    location: Location,
    frame: JvmStackFrame,
}

impl JvmFrameFact {
    fn new(location: Location, frame: JvmStackFrame) -> Self {
        let frame = if matches!(location, Location::Unwind) {
            frame.erase_values()
        } else {
            frame
        };
        Self { location, frame }
    }

    fn into_frame(self) -> JvmStackFrame {
        self.frame
    }
}

impl crate::analysis::fixed_point::JoinSemiLattice for JvmFrameFact {
    fn join_assign(&mut self, other: Self) -> bool {
        assert_eq!(self.location, other.location);
        let location = self.location;
        self.frame
            .join_assign_values_with(other.frame, |slot, lhs, rhs| {
                if *lhs == rhs {
                    return false;
                }
                let merged = MergeIdentity { location, slot };
                let value = match (*lhs, rhs) {
                    (OperandState::Invalid | OperandState::ReturnAddress(_), _)
                    | (_, OperandState::Invalid | OperandState::ReturnAddress(_)) => {
                        OperandState::Invalid
                    }
                    (OperandState::Merged(identity), _) if identity == merged => return false,
                    _ => OperandState::Merged(merged),
                };
                if *lhs == value {
                    false
                } else {
                    *lhs = value;
                    true
                }
            })
    }
}

impl PartialOrd for JvmFrameFact {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::{Equal, Greater, Less};

        if self.location != other.location {
            return None;
        }
        let mut lhs = self.clone();
        let mut rhs = other.clone();
        let lhs_changes =
            crate::analysis::fixed_point::JoinSemiLattice::join_assign(&mut lhs, other.clone());
        let rhs_changes =
            crate::analysis::fixed_point::JoinSemiLattice::join_assign(&mut rhs, self.clone());
        match (lhs_changes, rhs_changes) {
            (false, false) => Some(Equal),
            (false, true) => Some(Greater),
            (true, false) => Some(Less),
            (true, true) => None,
        }
    }
}

/// Completed abstract-execution facts for one reachable JVM location.
pub(in crate::ir::generator) struct AnalyzedLocation {
    pub incoming: JvmStackFrame,
    pub instruction: Instruction,
    pub is_explicit_transfer: bool,
    pub outgoing: Vec<JvmOutgoing>,
    pub caught_exception: Option<SsaValueId>,
}

/// Reachable JVM locations and abstract control-flow facts.
pub(in crate::ir::generator) struct AnalyzedJvmCfg {
    pub entry_location: Location,
    /// The original entry frame, retained only when a backedge needs a preheader input.
    pub initial_frame: Option<JvmStackFrame>,
    pub locations: BTreeMap<Location, AnalyzedLocation>,
    pub phi_values: BTreeMap<MergeIdentity, SsaValueId>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

impl DataflowProblem for JvmFrameAnalyzer<'_> {
    type Location = Location;
    type Fact = JvmFrameFact;
    type Err = MokaIRBuildError;
    type Output = JvmFlowOutput;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        self.entry.clone().into_iter().collect::<Vec<_>>()
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::Output, Self::Err> {
        let location = *location;
        if fact.location != location {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        let incoming = fact.frame.clone();
        let (instruction, outgoing) = match location {
            Location::Handler {
                handler_pc,
                context,
            } => {
                let target = self.normalizer.bytecode(handler_pc, context)?;
                (
                    Instruction::HandlerEntry,
                    vec![(
                        target,
                        ControlTransfer::Unconditional,
                        incoming.same_frame(),
                    )],
                )
            }
            Location::Unwind => (Instruction::Unwind, Vec::new()),
            Location::Bytecode { pc, .. } => {
                let pre_frame = incoming.same_frame();
                let mut normal_frame = incoming.same_frame();
                let jvm_instruction = self
                    .body
                    .instruction_at(pc)
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?
                    .clone();
                let fallible = self.fallibility.is_synchronously_fallible(&jvm_instruction);
                let instruction =
                    lift_instruction(self, &jvm_instruction, location, &mut normal_frame)?;
                let outgoing = outgoing_from(
                    self,
                    location,
                    &pre_frame,
                    normal_frame,
                    &instruction,
                    fallible,
                    &OperandState::Value,
                )?;
                (instruction, outgoing)
            }
        };

        let is_explicit_transfer = instruction.is_explicit_transfer();
        Ok(JvmFlowOutput {
            instruction,
            is_explicit_transfer,
            outgoing: outgoing
                .into_iter()
                .map(|(target, transfer, frame)| JvmFlowOutgoing {
                    target,
                    transfer,
                    frame: JvmFrameFact::new(target, frame),
                })
                .collect(),
        })
    }
}

impl<'method> JvmFrameAnalyzer<'method> {
    pub(super) fn for_method(method: &'method Method) -> Result<Self, MokaIRBuildError> {
        let body = method.body.as_ref().ok_or(MokaIRBuildError::NoMethodBody)?;
        let first_pc = body
            .instructions
            .entry_point()
            .map(|(pc, _)| *pc)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let mut analyzer = Self {
            body,
            fallibility: FallibilityContext::for_method(method),
            normalizer: Normalizer::new(first_pc),
            definition_ids: BTreeMap::new(),
            caught_exception_ids: BTreeMap::new(),
            next_value_index: 0,
            this_value: None,
            parameter_values: Vec::new(),
            entry: None,
        };
        analyzer.this_value = (!method.access_flags.contains(method::AccessFlags::STATIC))
            .then(|| analyzer.next_value_id())
            .transpose()?;
        analyzer.parameter_values = method
            .descriptor
            .parameters_types
            .iter()
            .map(|_| analyzer.next_value_id())
            .collect::<Result<_, _>>()?;
        let frame_parameters = analyzer
            .parameter_values
            .iter()
            .copied()
            .map(OperandState::Value)
            .collect::<Vec<_>>();
        let initial_frame = JvmStackFrame::with_inputs(
            &method.descriptor,
            body.max_locals,
            body.max_stack,
            analyzer.this_value.map(OperandState::Value),
            &frame_parameters,
        )?;
        let entry_location = Location::entry(first_pc);
        analyzer.entry = Some((
            entry_location,
            JvmFrameFact::new(entry_location, initial_frame),
        ));
        Ok(analyzer)
    }

    pub(super) fn run(mut self) -> Result<AnalyzedJvmCfg, MokaIRBuildError> {
        use crate::analysis::fixed_point::{FixedPointResult, solve_with_recomputed_outputs};

        let result: FixedPointResult<
            BTreeMap<Location, JvmFrameFact>,
            BTreeMap<Location, JvmFlowOutput>,
        > = solve_with_recomputed_outputs(&mut self)?;
        let (frames, mut outputs) = result.into_parts();
        let merge_identities = frames
            .values()
            .flat_map(|fact| fact.frame.values())
            .filter_map(|value| match value {
                OperandState::Merged(identity) => Some(*identity),
                OperandState::Value(_) | OperandState::ReturnAddress(_) | OperandState::Invalid => {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        let phi_values = merge_identities
            .into_iter()
            .map(|identity| self.next_value_id().map(|value| (identity, value)))
            .collect::<Result<_, _>>()?;
        let locations: BTreeMap<Location, AnalyzedLocation> = frames
            .into_iter()
            .map(|(location, fact)| {
                let output = outputs
                    .remove(&location)
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?;
                Ok((
                    location,
                    AnalyzedLocation {
                        incoming: fact.into_frame(),
                        instruction: output.instruction,
                        is_explicit_transfer: output.is_explicit_transfer,
                        outgoing: output
                            .outgoing
                            .into_iter()
                            .map(|outgoing| JvmOutgoing {
                                target: outgoing.target,
                                transfer: outgoing.transfer,
                                frame: outgoing.frame.into_frame(),
                            })
                            .collect(),
                        caught_exception: self.caught_exception_ids.get(&location).copied(),
                    },
                ))
            })
            .collect::<Result<_, MokaIRBuildError>>()?;
        if !outputs.is_empty() {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        let (entry_location, initial_frame) = self
            .entry
            .take()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let needs_entry_preheader = locations
            .values()
            .flat_map(|location| &location.outgoing)
            .any(|outgoing| outgoing.target == entry_location);
        Ok(AnalyzedJvmCfg {
            entry_location,
            initial_frame: needs_entry_preheader.then(|| initial_frame.into_frame()),
            locations,
            phi_values,
            this_value: self.this_value,
            parameter_values: self.parameter_values,
        })
    }

    fn next_value_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        let id = SsaValueId::new(self.next_value_index);
        self.next_value_index = self
            .next_value_index
            .checked_add(1)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        Ok(id)
    }
}

impl JvmSemantics for JvmFrameAnalyzer<'_> {
    fn body(&self) -> &MethodBody {
        self.body
    }

    fn definition_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(location, Location::Bytecode { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.definition_ids.get(&location) {
            return Ok(id);
        }
        let id = self.next_value_id()?;
        self.definition_ids.insert(location, id);
        Ok(id)
    }

    fn caught_exception_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(location, Location::Handler { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.caught_exception_ids.get(&location) {
            return Ok(id);
        }
        let id = self.next_value_id()?;
        self.caught_exception_ids.insert(location, id);
        Ok(id)
    }

    fn next_location(&mut self, location: Location) -> Result<Location, MokaIRBuildError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.bytecode(self.next_pc_of(pc)?, context)
    }

    fn target_location(
        &mut self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.bytecode(target, context)
    }

    fn handler_location(
        &mut self,
        location: Location,
        handler: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.handler(handler, context)
    }

    fn unwind_location(&mut self) -> Result<Location, MokaIRBuildError> {
        self.normalizer.register(Location::Unwind)
    }

    fn enter_subroutine(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBuildError> {
        self.normalizer.enter(location, target, continuation)
    }

    fn return_from(
        &mut self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBuildError> {
        self.normalizer.return_from(location, address)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        analysis::fixed_point::{JoinSemiLattice, solve},
        ir::generator::{
            jvm_frame::{Entry, FrameSlot},
            tests::method,
        },
        jvm::code::Instruction as JvmInstruction,
    };

    #[test]
    fn merge_identity_is_stable_for_a_location_and_slot() {
        let descriptor = "(I)V".parse().expect("valid descriptor");
        let location = Location::entry(0.into());
        let frame = |value| {
            JvmStackFrame::with_inputs(
                &descriptor,
                1,
                0,
                None,
                &[OperandState::Value(SsaValueId::new(value))],
            )
            .expect("frame fits descriptor")
        };
        let mut merged = JvmFrameFact::new(location, frame(1));

        assert!(merged.join_assign(JvmFrameFact::new(location, frame(2))));
        let expected = OperandState::Merged(MergeIdentity {
            location,
            slot: FrameSlot::Local(0),
        });
        assert_eq!(merged.frame.local_variables(), &[Entry::Value(expected)]);
        assert!(!merged.join_assign(JvmFrameFact::new(location, frame(3))));
        assert_eq!(merged.frame.local_variables(), &[Entry::Value(expected)]);
    }

    #[test]
    fn unwind_facts_discard_irrelevant_values_before_merging() {
        let descriptor = "(I)V".parse().expect("valid descriptor");
        let frame = |value| {
            JvmStackFrame::with_inputs(
                &descriptor,
                1,
                0,
                None,
                &[OperandState::Value(SsaValueId::new(value))],
            )
            .expect("frame fits descriptor")
        };
        let mut unwind = JvmFrameFact::new(Location::Unwind, frame(1));

        assert!(unwind.frame.values().next().is_none());
        assert!(!unwind.join_assign(JvmFrameFact::new(Location::Unwind, frame(2))));
        assert!(unwind.frame.values().next().is_none());
    }

    #[test]
    fn reprocessing_loop_allocates_identities_only_for_definitions() {
        let method = method(
            [
                (0.into(), JvmInstruction::IConst0),
                (1.into(), JvmInstruction::IStore0),
                (2.into(), JvmInstruction::ILoad0),
                (3.into(), JvmInstruction::IConst1),
                (4.into(), JvmInstruction::IAdd),
                (5.into(), JvmInstruction::IStore0),
                (6.into(), JvmInstruction::Goto(2.into())),
            ],
            "()V",
            vec![],
        );
        let mut analyzer = JvmFrameAnalyzer::for_method(&method).expect("valid method");
        let _: BTreeMap<Location, JvmFrameFact> = solve(&mut analyzer).expect("valid loop");

        assert_eq!(analyzer.definition_ids.len(), 3);
        assert_eq!(analyzer.next_value_index, 3);
        assert!(
            analyzer
                .definition_ids
                .contains_key(&Location::entry(0.into()))
        );
        assert!(
            analyzer
                .definition_ids
                .contains_key(&Location::entry(3.into()))
        );
        assert!(
            analyzer
                .definition_ids
                .contains_key(&Location::entry(4.into()))
        );
        assert!(
            !analyzer
                .definition_ids
                .contains_key(&Location::entry(1.into()))
        );
        assert!(
            !analyzer
                .definition_ids
                .contains_key(&Location::entry(2.into()))
        );
        assert!(
            !analyzer
                .definition_ids
                .contains_key(&Location::entry(5.into()))
        );
        assert!(
            !analyzer
                .definition_ids
                .contains_key(&Location::entry(6.into()))
        );
    }
}
