use super::{
    BTreeMap, BTreeSet, BlockId, FrameOperand, GeneratedMethod, HashMap, Identifier, JvmStackFrame,
    LiftedControlTransfer, LiftedInstruction, Location, MokaIRBrewingError, MokaIRGenerator,
    Operand, PlannedBlock, ScalarArm, ScalarBlock, ScalarEntryFrames, ScalarValue, ValueId,
    collect_phi_candidates, fallibility, method, next_temp_value, ssa, unavailable_value_slots,
};

impl MokaIRGenerator<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "block partitioning and identity allocation form one invariant-preserving pass"
    )]
    pub(super) fn assemble_blocks(
        mut self,
        facts: &HashMap<Location, JvmStackFrame>,
    ) -> Result<GeneratedMethod, MokaIRBrewingError> {
        let entry_location = self
            .initial_seed
            .as_ref()
            .map(|(location, _)| *location)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let reachable = self.lifted.keys().copied().collect::<Vec<_>>();
        if reachable.is_empty() {
            return Err(MokaIRBrewingError::MalformedControlFlow);
        }

        let mut leaders = BTreeSet::from([entry_location]);
        leaders.extend(
            reachable
                .iter()
                .copied()
                .filter(|location| !matches!(location, Location::Bytecode { .. })),
        );
        let mut predecessors: BTreeMap<Location, BTreeSet<Location>> = BTreeMap::new();
        for (&source, arms) in &self.outgoing {
            for (target, _) in arms {
                predecessors.entry(*target).or_default().insert(source);
            }
        }
        leaders.extend(
            predecessors
                .iter()
                .filter(|(_, sources)| sources.len() > 1)
                .map(|(target, _)| *target),
        );

        for (location, instruction) in &self.lifted {
            let arms = self.outgoing.get(location).map_or(
                &[] as &[(Location, LiftedControlTransfer<Operand>)],
                Vec::as_slice,
            );
            if instruction.is_explicit_transfer()
                || arms.iter().any(|(_, transfer)| {
                    matches!(
                        transfer,
                        LiftedControlTransfer::Normal
                            | LiftedControlTransfer::Exception(_)
                            | LiftedControlTransfer::Unwind
                    )
                })
            {
                leaders.extend(arms.iter().map(|(target, _)| *target));
            }
        }

        for pair in reachable.windows(2) {
            let [current, next] = pair else {
                unreachable!()
            };
            let instruction = self
                .lifted
                .get(current)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            let arms = self.outgoing.get(current).map_or(
                &[] as &[(Location, LiftedControlTransfer<Operand>)],
                Vec::as_slice,
            );
            let plain_fallthrough = !instruction.is_explicit_transfer()
                && arms.len() == 1
                && arms[0].0 == *next
                && matches!(arms[0].1, LiftedControlTransfer::Unconditional);
            if !plain_fallthrough {
                leaders.insert(*next);
            }
        }
        leaders.retain(|location| self.lifted.contains_key(location));

        let needs_entry_preheader = predecessors
            .get(&entry_location)
            .is_some_and(|sources| !sources.is_empty());
        let block_offset = u32::from(needs_entry_preheader);
        let block_ids = leaders
            .iter()
            .enumerate()
            .map(|(index, location)| {
                u32::try_from(index)
                    .ok()
                    .and_then(|index| index.checked_add(block_offset))
                    .map(|index| (*location, BlockId::new(index)))
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let bytecode_entry = *block_ids
            .get(&entry_location)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let entry = if needs_entry_preheader {
            BlockId::new(0)
        } else {
            bytecode_entry
        };

        let mut location_to_block = BTreeMap::new();
        let mut grouped: BTreeMap<BlockId, Vec<Location>> = BTreeMap::new();
        let mut current_block = None;
        for location in reachable {
            if let Some(id) = block_ids.get(&location) {
                current_block = Some(*id);
            }
            let id = current_block.ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            location_to_block.insert(location, id);
            grouped.entry(id).or_default().push(location);
        }
        let plans = grouped
            .into_iter()
            .map(|(id, pcs)| PlannedBlock { id, pcs })
            .collect::<Vec<_>>();

        let mut next_temp = self
            .value_ids
            .values()
            .chain(self.caught_exception_ids.values())
            .map(|value| value.index())
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let this_temp = if self
            .method
            .access_flags
            .contains(method::AccessFlags::STATIC)
        {
            None
        } else {
            Some(next_temp_value(&mut next_temp)?)
        };
        let parameter_temps = self
            .method
            .descriptor
            .parameters_types
            .iter()
            .map(|_| next_temp_value(&mut next_temp))
            .collect::<Result<Vec<_>, _>>()?;
        let scalar_this = this_temp.map(ScalarValue::Value);
        let scalar_parameters = parameter_temps
            .iter()
            .copied()
            .map(ScalarValue::Value)
            .collect::<Vec<_>>();
        let initial_scalar_frame = JvmStackFrame::with_inputs(
            &self.method.descriptor,
            self.body.max_locals,
            self.body.max_stack,
            scalar_this,
            &scalar_parameters,
        )?;

        let (entry_frames, phi_blocks) = self.scalar_entry_frames(
            &plans,
            facts,
            bytecode_entry,
            needs_entry_preheader,
            &initial_scalar_frame,
            this_temp,
            &parameter_temps,
            &mut next_temp,
        )?;
        let scalar_blocks =
            self.translate_scalar_blocks(&plans, entry_frames, &location_to_block)?;
        let candidates = collect_phi_candidates(
            &scalar_blocks,
            &phi_blocks,
            needs_entry_preheader.then_some((bytecode_entry, &initial_scalar_frame)),
        )?;
        let simplified =
            ssa::simplify_phis(candidates).map_err(|_| MokaIRBrewingError::MalformedControlFlow)?;
        self.materialize_scalar_method(
            entry,
            bytecode_entry,
            needs_entry_preheader,
            scalar_blocks,
            &phi_blocks,
            &simplified,
            this_temp,
            &parameter_temps,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "entry-state construction joins method inputs, discovery facts, and deterministic allocation"
    )]
    fn scalar_entry_frames(
        &self,
        plans: &[PlannedBlock],
        facts: &HashMap<Location, JvmStackFrame>,
        bytecode_entry: BlockId,
        needs_entry_preheader: bool,
        initial_frame: &JvmStackFrame<ScalarValue>,
        this_temp: Option<ValueId>,
        parameter_temps: &[ValueId],
        next_temp: &mut u32,
    ) -> Result<ScalarEntryFrames, MokaIRBrewingError> {
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
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            let discovered = facts
                .get(&leader)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            if matches!(leader, Location::Unwind) {
                let frame = discovered.without_values().try_map_values(
                    |_| -> Result<ScalarValue, MokaIRBrewingError> {
                        Err(MokaIRBrewingError::MalformedControlFlow)
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
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                incoming.push(initial);
            }
            let (unavailable_locals, unavailable_stack) =
                unavailable_value_slots(discovered, &incoming)?;
            let mut frame = discovered.try_map_values(|operand| {
                if operand.0.len() == 1 {
                    let identifier = *operand
                        .0
                        .first()
                        .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                    return Ok(match identifier {
                        Identifier::This => ScalarValue::Value(
                            this_temp.ok_or(MokaIRBrewingError::MalformedControlFlow)?,
                        ),
                        Identifier::Arg(index) => ScalarValue::Value(
                            parameter_temps
                                .get(usize::from(index))
                                .copied()
                                .ok_or(MokaIRBrewingError::MalformedControlFlow)?,
                        ),
                        Identifier::Local(value) | Identifier::CaughtException(value) => {
                            ScalarValue::Value(value)
                        }
                        Identifier::ReturnAddress(address) => ScalarValue::ReturnAddress(address),
                    });
                }
                if operand.contains_return_address() {
                    return Err(MokaIRBrewingError::MalformedControlFlow);
                }
                let value = next_temp_value(next_temp)?;
                phi_blocks.insert(value, plan.id);
                Ok(ScalarValue::Value(value))
            })?;
            frame.invalidate_values_at(unavailable_locals, unavailable_stack);
            frames.insert(plan.id, frame);
        }
        Ok((frames, phi_blocks))
    }

    fn incoming_frames_at(
        &self,
        target: Location,
    ) -> Result<Vec<&JvmStackFrame>, MokaIRBrewingError> {
        let mut incoming = Vec::new();
        for (source, arms) in &self.outgoing {
            let frames = self
                .outgoing_frames
                .get(source)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            if arms.len() != frames.len() {
                return Err(MokaIRBrewingError::MalformedControlFlow);
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

    fn translate_scalar_blocks(
        &mut self,
        plans: &[PlannedBlock],
        mut entry_frames: BTreeMap<BlockId, JvmStackFrame<ScalarValue>>,
        location_to_block: &BTreeMap<Location, BlockId>,
    ) -> Result<Vec<ScalarBlock>, MokaIRBrewingError> {
        let mut blocks = Vec::with_capacity(plans.len());
        for plan in plans {
            let entry_frame = entry_frames
                .remove(&plan.id)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
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
                            .ok_or(MokaIRBrewingError::MalformedControlFlow)?
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
                                .ok_or(MokaIRBrewingError::MalformedControlFlow)
                        })
                        .collect::<Result<_, _>>()?;
                } else if instruction.is_explicit_transfer() {
                    return Err(MokaIRBrewingError::MalformedControlFlow);
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
