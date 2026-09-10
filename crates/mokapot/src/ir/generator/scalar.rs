use super::{
    BTreeMap, BlockId, DiscoveryValue, JvmStackFrame, LiftedInstruction, Location,
    MokaIRBuildError, MokaIRGenerator, PlannedBlock, ProvisionalValueId, ScalarArm, ScalarBlock,
    ScalarEntryFrames, ScalarValue, fallibility, next_temp_value, unavailable_value_slots,
};

impl MokaIRGenerator<'_> {
    #[expect(
        clippy::too_many_arguments,
        reason = "entry-state construction joins method inputs, discovery facts, and deterministic allocation"
    )]
    pub(super) fn scalar_entry_frames(
        &self,
        plans: &[PlannedBlock],
        facts: &BTreeMap<Location, JvmStackFrame>,
        bytecode_entry: BlockId,
        needs_entry_preheader: bool,
        initial_frame: &JvmStackFrame<ScalarValue>,
        this_temp: Option<ProvisionalValueId>,
        parameter_temps: &[ProvisionalValueId],
        next_temp: &mut u32,
    ) -> Result<ScalarEntryFrames, MokaIRBuildError> {
        let mut frames = BTreeMap::new();
        let mut phi_blocks = BTreeMap::new();
        for plan in plans {
            if plan.id == bytecode_entry && !needs_entry_preheader {
                frames.insert(plan.id, initial_frame.clone());
                continue;
            }
            let leader = *plan
                .pcs
                .first()
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            let discovered = facts
                .get(&leader)
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            if matches!(leader, Location::Unwind) {
                let frame = discovered.without_values().try_map_values(
                    |_| -> Result<ScalarValue, MokaIRBuildError> {
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
                unavailable_value_slots(discovered, &incoming)?;
            let mut frame = discovered.try_map_values(|value| {
                Ok(match value {
                    DiscoveryValue::This => {
                        ScalarValue::Value(this_temp.ok_or(MokaIRBuildError::MalformedControlFlow)?)
                    }
                    DiscoveryValue::Arg(index) => ScalarValue::Value(
                        parameter_temps
                            .get(usize::from(*index))
                            .copied()
                            .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                    ),
                    DiscoveryValue::Local(value) | DiscoveryValue::CaughtException(value) => {
                        ScalarValue::Value(*value)
                    }
                    DiscoveryValue::ReturnAddress(address) => ScalarValue::ReturnAddress(*address),
                    DiscoveryValue::Merged => {
                        let value = next_temp_value(next_temp)?;
                        phi_blocks.insert(value, plan.id);
                        ScalarValue::Value(value)
                    }
                    DiscoveryValue::Invalid => {
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

    pub(super) fn translate_scalar_blocks(
        &mut self,
        plans: &[PlannedBlock],
        mut entry_frames: BTreeMap<BlockId, JvmStackFrame<ScalarValue>>,
        location_to_block: &BTreeMap<Location, BlockId>,
    ) -> Result<Vec<ScalarBlock>, MokaIRBuildError> {
        let mut blocks = Vec::with_capacity(plans.len());
        for plan in plans {
            let entry_frame = entry_frames
                .remove(&plan.id)
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            let mut frame = entry_frame.clone();
            let mut instructions = Vec::with_capacity(plan.pcs.len());
            let mut arms = Vec::new();
            for (index, &location) in plan.pcs.iter().enumerate() {
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
                            fallibility::is_synchronously_fallible(&jvm_instruction),
                        )
                    }
                    Location::Handler { .. } => (LiftedInstruction::HandlerEntry, false),
                    Location::Unwind => (LiftedInstruction::Unwind, false),
                };
                let is_last = index + 1 == plan.pcs.len();
                if is_last {
                    arms = self
                        .analyze_frame_and_conditions(
                            location,
                            &pre_frame,
                            frame.clone(),
                            &instruction,
                            fallible,
                            &|value| ScalarValue::Value(value),
                        )?
                        .into_iter()
                        .map(|(target, transfer, frame)| {
                            location_to_block
                                .get(&target)
                                .copied()
                                .map(|target| ScalarArm {
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
            blocks.push(ScalarBlock {
                plan: plan.clone(),
                entry_frame,
                instructions,
                arms,
            });
        }
        Ok(blocks)
    }
}
