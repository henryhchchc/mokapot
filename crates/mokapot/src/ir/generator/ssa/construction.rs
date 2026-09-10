use super::super::block_formation::BlockPlan;
use super::{
    BTreeMap, BlockId, JvmFrameAnalysis, JvmStackFrame, LiftedInstruction, Location,
    MokaIRBuildError, OperandState, SsaArm, SsaBlock, SsaEntryFrames, SsaFrameValue, SsaValueId,
    next_ssa_value, unavailable_value_slots,
};

impl JvmFrameAnalysis<'_> {
    #[expect(
        clippy::too_many_arguments,
        reason = "entry-state construction joins method inputs, analysis facts, and deterministic allocation"
    )]
    pub(in crate::ir::generator) fn ssa_entry_frames(
        &self,
        plans: &[BlockPlan],
        facts: &BTreeMap<Location, JvmStackFrame>,
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
            let analyzed = facts
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
                let (_, initial) = self
                    .initial_seed
                    .as_ref()
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?;
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
        for (source, arms) in &self.outgoing {
            let frames = self
                .outgoing_frames
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

    pub(in crate::ir::generator) fn construct_ssa_blocks(
        &mut self,
        plans: &[BlockPlan],
        mut entry_frames: BTreeMap<BlockId, JvmStackFrame<SsaFrameValue>>,
        location_to_block: &BTreeMap<Location, BlockId>,
    ) -> Result<Vec<SsaBlock>, MokaIRBuildError> {
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
                            .body
                            .instruction_at(pc)
                            .ok_or(MokaIRBuildError::MalformedControlFlow)?
                            .clone();
                        let instruction =
                            self.lift_instruction(&jvm_instruction, location, &mut frame)?;
                        (
                            instruction,
                            self.fallibility.is_synchronously_fallible(&jvm_instruction),
                        )
                    }
                    Location::Handler { .. } => (LiftedInstruction::HandlerEntry, false),
                    Location::Unwind => (LiftedInstruction::Unwind, false),
                };
                let is_last = index + 1 == plan.locations.len();
                if is_last {
                    arms = self
                        .analyze_frame_and_conditions(
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
