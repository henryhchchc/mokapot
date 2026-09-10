use super::super::block_formation::BlockPlan;
use super::{
    BTreeMap, BlockId, JvmFrameFacts, JvmStackFrame, LiftedInstruction, Location, MokaIRBuildError,
    OperandState, ReturnAddress, SsaArm, SsaBlock, SsaEntryFrames, SsaFrameValue, SsaValueId,
    next_ssa_value, unavailable_value_slots,
};
use crate::ir::generator::lifting::{
    fallibility::FallibilityContext,
    lift_instruction,
    semantics::{JvmSemantics, outgoing_from},
};
use crate::jvm::code::{MethodBody, ProgramCounter};

pub(super) struct SsaBuilder<'analysis, 'method> {
    frame_facts: &'analysis JvmFrameFacts<'method>,
}

impl<'analysis, 'method> SsaBuilder<'analysis, 'method> {
    pub(super) const fn new(frame_facts: &'analysis JvmFrameFacts<'method>) -> Self {
        Self { frame_facts }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "entry-state construction joins method inputs, analysis facts, and deterministic allocation"
    )]
    pub(super) fn entry_frames(
        &self,
        plans: &[BlockPlan],
        bytecode_entry: BlockId,
        needs_entry_preheader: bool,
        initial_frame: &JvmStackFrame<SsaFrameValue>,
        this_temp: Option<SsaValueId>,
        parameter_temps: &[SsaValueId],
        next_temp: &mut u32,
    ) -> Result<SsaEntryFrames, MokaIRBuildError> {
        let mut frames = BTreeMap::new();
        let mut phi_blocks = BTreeMap::new();
        for plan in plans {
            if plan.id == bytecode_entry && !needs_entry_preheader {
                frames.insert(plan.id, initial_frame.clone());
                continue;
            }
            let leader = *plan
                .locations
                .first()
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            let analyzed = self
                .frame_facts
                .frames
                .get(&leader)
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            if matches!(leader, Location::Unwind) {
                let frame = analyzed.without_values().try_map_values(
                    |_| -> Result<SsaFrameValue, MokaIRBuildError> {
                        Err(MokaIRBuildError::MalformedControlFlow)
                    },
                )?;
                frames.insert(plan.id, frame);
                continue;
            }
            let mut incoming = self.incoming_frames_at(leader)?;
            if plan.id == bytecode_entry && needs_entry_preheader {
                let initial = &self.frame_facts.entry.1;
                incoming.push(initial);
            }
            let (unavailable_locals, unavailable_stack) =
                unavailable_value_slots(analyzed, &incoming)?;
            let mut frame = analyzed.try_map_values(|value| {
                Ok(match value {
                    OperandState::This => SsaFrameValue::Value(
                        this_temp.ok_or(MokaIRBuildError::MalformedControlFlow)?,
                    ),
                    OperandState::Arg(index) => SsaFrameValue::Value(
                        parameter_temps
                            .get(usize::from(*index))
                            .copied()
                            .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                    ),
                    OperandState::Local(value) | OperandState::CaughtException(value) => {
                        SsaFrameValue::Value(*value)
                    }
                    OperandState::ReturnAddress(address) => SsaFrameValue::ReturnAddress(*address),
                    OperandState::Merged => {
                        let value = next_ssa_value(next_temp)?;
                        phi_blocks.insert(value, plan.id);
                        SsaFrameValue::Value(value)
                    }
                    OperandState::Invalid => {
                        return Err(MokaIRBuildError::MalformedControlFlow);
                    }
                })
            })?;
            frame.invalidate_values_at(unavailable_locals, unavailable_stack);
            frames.insert(plan.id, frame);
        }
        Ok((frames, phi_blocks))
    }

    fn incoming_frames_at(
        &self,
        target: Location,
    ) -> Result<Vec<&JvmStackFrame>, MokaIRBuildError> {
        let mut incoming = Vec::new();
        for (source, arms) in &self.frame_facts.successors {
            let frames = self
                .frame_facts
                .successor_frames
                .get(source)
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            if arms.len() != frames.len() {
                return Err(MokaIRBuildError::MalformedControlFlow);
            }
            incoming.extend(
                arms.iter()
                    .zip(frames)
                    .filter(|((arm_target, _), _)| *arm_target == target)
                    .map(|(_, frame)| frame),
            );
        }
        Ok(incoming)
    }

    pub(super) fn construct_blocks(
        &self,
        plans: &[BlockPlan],
        mut entry_frames: BTreeMap<BlockId, JvmStackFrame<SsaFrameValue>>,
        location_to_block: &BTreeMap<Location, BlockId>,
    ) -> Result<Vec<SsaBlock>, MokaIRBuildError> {
        let mut replay = SsaReplay::new(self.frame_facts);
        let fallibility = FallibilityContext::for_method(self.frame_facts.method);
        let mut blocks = Vec::with_capacity(plans.len());
        for plan in plans {
            let entry_frame = entry_frames
                .remove(&plan.id)
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            let mut frame = entry_frame.clone();
            let mut instructions = Vec::with_capacity(plan.locations.len());
            let mut arms = Vec::new();
            for (index, &location) in plan.locations.iter().enumerate() {
                let pre_frame = frame.clone();
                let (instruction, fallible) = match location {
                    Location::Bytecode { pc, .. } => {
                        let jvm_instruction = self
                            .frame_facts
                            .body
                            .instruction_at(pc)
                            .ok_or(MokaIRBuildError::MalformedControlFlow)?
                            .clone();
                        let instruction =
                            lift_instruction(&mut replay, &jvm_instruction, location, &mut frame)?;
                        (
                            instruction,
                            fallibility.is_synchronously_fallible(&jvm_instruction),
                        )
                    }
                    Location::Handler { .. } => (LiftedInstruction::HandlerEntry, false),
                    Location::Unwind => (LiftedInstruction::Unwind, false),
                };
                let is_last = index + 1 == plan.locations.len();
                if is_last {
                    arms = outgoing_from(
                        &mut replay,
                        location,
                        &pre_frame,
                        frame.clone(),
                        &instruction,
                        fallible,
                        &|value| SsaFrameValue::Value(value),
                    )?
                    .into_iter()
                    .map(|(target, transfer, frame)| {
                        location_to_block
                            .get(&target)
                            .copied()
                            .map(|target| SsaArm {
                                target,
                                transfer,
                                frame,
                            })
                            .ok_or(MokaIRBuildError::MalformedControlFlow)
                    })
                    .collect::<Result<_, _>>()?;
                } else if instruction.is_explicit_transfer() {
                    return Err(MokaIRBuildError::MalformedControlFlow);
                }
                instructions.push((location, instruction));
            }
            blocks.push(SsaBlock {
                plan: plan.clone(),
                entry_frame,
                instructions,
                arms,
            });
        }
        Ok(blocks)
    }
}

struct SsaReplay<'facts, 'method> {
    frame_facts: &'facts JvmFrameFacts<'method>,
}

impl<'facts, 'method> SsaReplay<'facts, 'method> {
    const fn new(frame_facts: &'facts JvmFrameFacts<'method>) -> Self {
        Self { frame_facts }
    }
}

impl JvmSemantics for SsaReplay<'_, '_> {
    fn body(&self) -> &MethodBody {
        self.frame_facts.body
    }

    fn definition_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        self.frame_facts
            .definition_ids
            .get(&location)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }

    fn caught_exception_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        self.frame_facts
            .caught_exception_ids
            .get(&location)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }

    fn next_location(&mut self, location: Location) -> Result<Location, MokaIRBuildError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.frame_facts
            .normalized
            .bytecode(self.next_pc_of(pc)?, context)
    }

    fn target_location(
        &mut self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.frame_facts.normalized.bytecode(target, context)
    }

    fn handler_location(
        &mut self,
        location: Location,
        handler: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.frame_facts.normalized.handler(handler, context)
    }

    fn unwind_location(&mut self) -> Result<Location, MokaIRBuildError> {
        self.frame_facts.normalized.unwind()
    }

    fn enter_subroutine(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBuildError> {
        self.frame_facts
            .normalized
            .enter(location, target, continuation)
    }

    fn return_from(
        &mut self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBuildError> {
        self.frame_facts.normalized.return_from(location, address)
    }
}
