use super::{
    BTreeMap, BTreeSet, BlockId, GeneratedMethod, HashMap, Identifier, JvmStackFrame,
    LiftedControlTransfer, LiftedInstruction, MokaIRBrewingError, MokaIRGenerator, Operand,
    PlannedBlock, ProgramCounter, ScalarArm, ScalarBlock, ScalarEntryFrames, ValueId,
    collect_phi_candidates, method, next_temp_value, ssa, unavailable_value_slots,
};

impl MokaIRGenerator<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "block partitioning and identity allocation form one invariant-preserving pass"
    )]
    pub(super) fn assemble_blocks(
        mut self,
        facts: &HashMap<ProgramCounter, JvmStackFrame>,
    ) -> Result<GeneratedMethod, MokaIRBrewingError> {
        let entry_pc = self
            .initial_seed
            .as_ref()
            .map(|(pc, _)| *pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let reachable = self.lifted.keys().copied().collect::<Vec<_>>();
        if reachable.is_empty() {
            return Err(MokaIRBrewingError::MalformedControlFlow);
        }

        let mut leaders = BTreeSet::from([entry_pc]);
        leaders.extend(
            self.body
                .exception_table
                .iter()
                .map(|entry| entry.handler_pc)
                .filter(|handler| self.lifted.contains_key(handler)),
        );
        let mut predecessors: BTreeMap<ProgramCounter, BTreeSet<ProgramCounter>> = BTreeMap::new();
        for (&source, arms) in &self.outgoing {
            for (target, transfer) in arms {
                predecessors.entry(*target).or_default().insert(source);
                if matches!(transfer, LiftedControlTransfer::Exception(_)) {
                    leaders.insert(*target);
                }
            }
        }
        leaders.extend(
            predecessors
                .iter()
                .filter(|(_, sources)| sources.len() > 1)
                .map(|(target, _)| *target),
        );

        for (pc, instruction) in &self.lifted {
            let arms = self.outgoing.get(pc).map_or(
                &[] as &[(ProgramCounter, LiftedControlTransfer<Operand>)],
                Vec::as_slice,
            );
            if instruction.is_explicit_transfer()
                || arms
                    .iter()
                    .any(|(_, transfer)| matches!(transfer, LiftedControlTransfer::Exception(_)))
            {
                leaders.extend(arms.iter().map(|(target, _)| *target));
            }
            if let LiftedInstruction::Subroutine { return_address, .. } = instruction
                && self.lifted.contains_key(return_address)
            {
                leaders.insert(*return_address);
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
                &[] as &[(ProgramCounter, LiftedControlTransfer<Operand>)],
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
        leaders.retain(|pc| self.lifted.contains_key(pc));

        let needs_entry_preheader = predecessors
            .get(&entry_pc)
            .is_some_and(|sources| !sources.is_empty());
        let block_offset = u32::from(needs_entry_preheader);
        let block_ids = leaders
            .iter()
            .enumerate()
            .map(|(index, pc)| {
                u32::try_from(index)
                    .ok()
                    .and_then(|index| index.checked_add(block_offset))
                    .map(|index| (*pc, BlockId::new(index)))
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let bytecode_entry = *block_ids
            .get(&entry_pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let entry = if needs_entry_preheader {
            BlockId::new(0)
        } else {
            bytecode_entry
        };

        let mut pc_to_block = BTreeMap::new();
        let mut grouped: BTreeMap<BlockId, Vec<ProgramCounter>> = BTreeMap::new();
        let mut current_block = None;
        for pc in reachable {
            if let Some(id) = block_ids.get(&pc) {
                current_block = Some(*id);
            }
            let id = current_block.ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            pc_to_block.insert(pc, id);
            grouped.entry(id).or_default().push(pc);
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
        let initial_scalar_frame = JvmStackFrame::with_inputs(
            &self.method.descriptor,
            self.body.max_locals,
            self.body.max_stack,
            this_temp,
            &parameter_temps,
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
        let scalar_blocks = self.translate_scalar_blocks(&plans, entry_frames, &pc_to_block)?;
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
        facts: &HashMap<ProgramCounter, JvmStackFrame>,
        bytecode_entry: BlockId,
        needs_entry_preheader: bool,
        initial_frame: &JvmStackFrame<ValueId>,
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
                if let Some(identifier) = operand.0.iter().copied().next()
                    && operand.0.len() == 1
                {
                    return match identifier {
                        Identifier::This => {
                            this_temp.ok_or(MokaIRBrewingError::MalformedControlFlow)
                        }
                        Identifier::Arg(index) => parameter_temps
                            .get(usize::from(index))
                            .copied()
                            .ok_or(MokaIRBrewingError::MalformedControlFlow),
                        Identifier::Local(value) | Identifier::CaughtException(value) => Ok(value),
                    };
                }
                let value = next_temp_value(next_temp)?;
                phi_blocks.insert(value, plan.id);
                Ok(value)
            })?;
            frame.invalidate_values_at(unavailable_locals, unavailable_stack);
            frames.insert(plan.id, frame);
        }
        Ok((frames, phi_blocks))
    }

    fn incoming_frames_at(
        &self,
        target: ProgramCounter,
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
        mut entry_frames: BTreeMap<BlockId, JvmStackFrame<ValueId>>,
        pc_to_block: &BTreeMap<ProgramCounter, BlockId>,
    ) -> Result<Vec<ScalarBlock>, MokaIRBrewingError> {
        let mut blocks = Vec::with_capacity(plans.len());
        for plan in plans {
            let entry_frame = entry_frames
                .remove(&plan.id)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            let mut frame = entry_frame.clone();
            let mut instructions = Vec::with_capacity(plan.pcs.len());
            let mut arms = Vec::new();
            for (index, &pc) in plan.pcs.iter().enumerate() {
                let jvm_instruction = self
                    .body
                    .instruction_at(pc)
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                let instruction = self.lift_instruction(jvm_instruction, pc, &mut frame)?;
                let is_last = index + 1 == plan.pcs.len();
                if is_last {
                    arms = self
                        .analyze_frame_and_conditions(pc, frame.clone(), &instruction, &|value| {
                            value
                        })?
                        .into_iter()
                        .map(|(target, transfer, frame)| {
                            pc_to_block
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
                instructions.push((pc, instruction));
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
