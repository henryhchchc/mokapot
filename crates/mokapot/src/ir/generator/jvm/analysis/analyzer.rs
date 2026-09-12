use std::collections::{BTreeMap, BTreeSet};

use crate::{
    analysis::fixed_point::{DataflowProblem, FixedPointResult, solve_with_recomputed_outputs},
    ir::{
        control_flow::ControlTransfer,
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                analysis::fact::{
                    AnalyzedJvmCfg, AnalyzedLocation, JvmFlowOutgoing, JvmFlowOutput, JvmFrameFact,
                    JvmOutgoing, OperandState,
                },
                frame::JvmStackFrame,
                instruction::Instruction,
                lifting::{
                    fallibility::FallibilityContext, lift_instruction, semantics::outgoing_from,
                },
                normalization::{Location, Normalizer},
            },
        },
    },
    jvm::{Method, code::MethodBody, method},
};

/// Mutable state used only while solving JVM frame facts.
pub(in crate::ir::generator) struct JvmFrameAnalyzer<'method> {
    pub(super) body: &'method MethodBody,
    fallibility: FallibilityContext,
    pub(super) normalizer: Normalizer,
    pub(super) definition_ids: BTreeMap<Location, SsaValueId>,
    pub(super) caught_exception_ids: BTreeMap<Location, SsaValueId>,
    pub(super) value_id_allocator: ValueIdAllocator,
    this_value: Option<SsaValueId>,
    parameter_values: Vec<SsaValueId>,
    entry: JvmFrameFact,
}

impl DataflowProblem for JvmFrameAnalyzer<'_> {
    type Location = Location;
    type Fact = JvmFrameFact;
    type Err = MokaIRBuildError;
    type Output = JvmFlowOutput;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        std::iter::once((self.entry.location, self.entry.clone()))
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

        Ok(JvmFlowOutput {
            instruction,
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
    pub(in crate::ir::generator) fn for_method(
        method: &'method Method,
    ) -> Result<Self, MokaIRBuildError> {
        let body = method.body.as_ref().ok_or(MokaIRBuildError::NoMethodBody)?;
        let first_pc = body
            .instructions
            .entry_point()
            .map(|(pc, _)| *pc)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let mut value_id_allocator = ValueIdAllocator::default();
        let this_value = (!method.access_flags.contains(method::AccessFlags::STATIC))
            .then(|| value_id_allocator.new_value_id())
            .transpose()?;
        let parameter_values: Vec<SsaValueId> = method
            .descriptor
            .parameters_types
            .iter()
            .map(|_| value_id_allocator.new_value_id())
            .collect::<Result<_, _>>()?;
        let frame_parameters = parameter_values
            .iter()
            .copied()
            .map(OperandState::Value)
            .collect::<Vec<_>>();
        let initial_frame = JvmStackFrame::with_inputs(
            &method.descriptor,
            body.max_locals,
            body.max_stack,
            this_value.map(OperandState::Value),
            &frame_parameters,
        )?;
        let entry_location = Location::entry(first_pc);
        let entry = JvmFrameFact::new(entry_location, initial_frame);
        let analyzer = Self {
            body,
            fallibility: FallibilityContext::for_method(method),
            normalizer: Normalizer::new(first_pc),
            definition_ids: BTreeMap::new(),
            caught_exception_ids: BTreeMap::new(),
            value_id_allocator,
            this_value,
            parameter_values,
            entry,
        };
        Ok(analyzer)
    }

    pub(in crate::ir::generator) fn run(mut self) -> Result<AnalyzedJvmCfg, MokaIRBuildError> {
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
            .map(|identity| self.new_value_id().map(|value| (identity, value)))
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
        let entry_location = self.entry.location;
        let initial_frame = self.entry.into_frame();
        Ok(AnalyzedJvmCfg {
            entry_location,
            initial_frame,
            locations,
            phi_values,
            this_value: self.this_value,
            parameter_values: self.parameter_values,
        })
    }

    pub(super) fn new_value_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        self.value_id_allocator.new_value_id()
    }
}

#[derive(Debug, Default)]
pub(super) struct ValueIdAllocator {
    pub(super) next_value_idx: u32,
}

impl ValueIdAllocator {
    fn new_value_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        let id = SsaValueId::new(self.next_value_idx);
        self.next_value_idx = self
            .next_value_idx
            .checked_add(1)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        Ok(id)
    }
}
