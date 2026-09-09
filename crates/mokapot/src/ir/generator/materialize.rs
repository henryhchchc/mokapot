use super::{
    BTreeMap, BasicBlock, BlockId, ControlTransfer, EdgeId, GeneratedMethod, InstructionId,
    InstructionKind, LiftedInstruction, MokaIRBrewingError, MokaIRGenerator, MokaInstruction, Phi,
    PhiInput, ScalarBlock, ScalarValue, SourceMap, Successor, Terminator, TerminatorKind,
    ValueDefinition, ValueId, remap_expression, remap_transfer, ssa,
};

impl MokaIRGenerator<'_> {
    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "final SSA allocation and block materialization form one ordered pass"
    )]
    pub(super) fn materialize_scalar_method(
        &self,
        entry: BlockId,
        bytecode_entry: BlockId,
        needs_entry_preheader: bool,
        scalar_blocks: Vec<ScalarBlock>,
        phi_blocks: &BTreeMap<ValueId, BlockId>,
        simplified: &ssa::SimplifiedPhis,
        this_temp: Option<ValueId>,
        parameter_temps: &[ValueId],
    ) -> Result<GeneratedMethod, MokaIRBrewingError> {
        let mut next_instruction = 0_u32;
        let mut next_value = 0_u32;
        let mut temp_values = BTreeMap::new();
        let mut value_definitions = Vec::new();
        let mut phi_ids = BTreeMap::new();
        let mut instruction_ids = BTreeMap::new();
        let mut terminator_ids = BTreeMap::new();

        let this_value = this_temp
            .map(|temp| {
                allocate_final_value(
                    temp,
                    ValueDefinition::This,
                    &mut next_value,
                    &mut temp_values,
                    &mut value_definitions,
                )
            })
            .transpose()?;
        let parameter_values = parameter_temps
            .iter()
            .enumerate()
            .map(|(index, &temp)| {
                let index =
                    u16::try_from(index).map_err(|_| MokaIRBrewingError::MalformedControlFlow)?;
                allocate_final_value(
                    temp,
                    ValueDefinition::Parameter(index),
                    &mut next_value,
                    &mut temp_values,
                    &mut value_definitions,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        if needs_entry_preheader {
            terminator_ids.insert(entry, allocate_instruction_id(&mut next_instruction)?);
        }

        let mut caught_exceptions = BTreeMap::new();
        for block in &scalar_blocks {
            let leader = *block
                .plan
                .pcs
                .first()
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            if let Some(&temp) = self.caught_exception_ids.get(&leader) {
                let value = allocate_final_value(
                    temp,
                    ValueDefinition::CaughtException(block.plan.id),
                    &mut next_value,
                    &mut temp_values,
                    &mut value_definitions,
                )?;
                caught_exceptions.insert(block.plan.id, value);
            }

            for (&temp, &phi_block) in phi_blocks {
                if phi_block != block.plan.id || !simplified.candidates.contains_key(&temp) {
                    continue;
                }
                let id = allocate_instruction_id(&mut next_instruction)?;
                phi_ids.insert(temp, id);
                allocate_final_value(
                    temp,
                    ValueDefinition::Instruction(id),
                    &mut next_value,
                    &mut temp_values,
                    &mut value_definitions,
                )?;
            }

            for (location, instruction) in &block.instructions {
                let retained = matches!(
                    instruction,
                    LiftedInstruction::Definition { .. } | LiftedInstruction::Effect(_)
                );
                if !retained {
                    continue;
                }
                let id = allocate_instruction_id(&mut next_instruction)?;
                instruction_ids.insert(*location, id);
                if let LiftedInstruction::Definition { value, .. } = instruction {
                    allocate_final_value(
                        *value,
                        ValueDefinition::Instruction(id),
                        &mut next_value,
                        &mut temp_values,
                        &mut value_definitions,
                    )?;
                }
            }
            terminator_ids.insert(
                block.plan.id,
                allocate_instruction_id(&mut next_instruction)?,
            );
        }

        let remap = |value| resolve_final_value(value, &simplified.substitutions, &temp_values);
        let remap_operand = |value| match value {
            ScalarValue::Value(value) => remap(value),
            ScalarValue::ReturnAddress(_) => Err(MokaIRBrewingError::MalformedControlFlow),
        };
        let mut source_map = SourceMap::default();
        let mut blocks =
            Vec::with_capacity(scalar_blocks.len() + usize::from(needs_entry_preheader));
        let mut next_edge = 0_u32;
        if needs_entry_preheader {
            let successor = Successor::new(
                allocate_edge_id(&mut next_edge)?,
                bytecode_entry,
                ControlTransfer::Unconditional,
            );
            let terminator = Terminator::new(
                *terminator_ids
                    .get(&entry)
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?,
                TerminatorKind::Goto,
                vec![successor],
            );
            blocks.push(BasicBlock::new(entry, vec![], vec![], terminator));
        }

        for block in scalar_blocks {
            let mut phis = Vec::new();
            for (&temp, &phi_block) in phi_blocks {
                if phi_block != block.plan.id {
                    continue;
                }
                let Some(inputs) = simplified.candidates.get(&temp) else {
                    continue;
                };
                let id = *phi_ids
                    .get(&temp)
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                let value = remap(temp)?;
                let inputs = inputs
                    .iter()
                    .map(|&(predecessor, value)| {
                        remap(value).map(|value| PhiInput::new(predecessor, value))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                phis.push(Phi::new(id, value, inputs));
            }

            let mut instructions = Vec::new();
            for (location, lifted) in &block.instructions {
                let kind = match lifted.clone() {
                    LiftedInstruction::Definition { value, expr } => {
                        Some(InstructionKind::Definition {
                            value: remap(value)?,
                            expr: remap_expression(expr, &remap_operand)?,
                        })
                    }
                    LiftedInstruction::Effect(expr) => Some(InstructionKind::Effect {
                        expr: remap_expression(expr, &remap_operand)?,
                    }),
                    LiftedInstruction::HandlerEntry
                    | LiftedInstruction::Unwind
                    | LiftedInstruction::Nop
                    | LiftedInstruction::Subroutine { .. }
                    | LiftedInstruction::Jump { .. }
                    | LiftedInstruction::Switch { .. }
                    | LiftedInstruction::Return(_)
                    | LiftedInstruction::Throw(_)
                    | LiftedInstruction::SubroutineReturn(_) => None,
                };
                if let Some(kind) = kind {
                    let id = *instruction_ids
                        .get(location)
                        .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                    let pc = location
                        .source_pc()
                        .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                    source_map.insert(pc, id);
                    instructions.push(MokaInstruction::new(id, kind));
                }
            }

            let successors = block
                .arms
                .into_iter()
                .map(|arm| {
                    Ok(Successor::new(
                        allocate_edge_id(&mut next_edge)?,
                        arm.target,
                        remap_transfer(arm.transfer, &remap_operand)?,
                    ))
                })
                .collect::<Result<Vec<_>, MokaIRBrewingError>>()?;
            let (last_location, last) = block
                .instructions
                .last()
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            let source_backed = last.is_explicit_transfer() && last_location.source_pc().is_some();
            let kind = match last {
                LiftedInstruction::Unwind => TerminatorKind::Unwind,
                LiftedInstruction::Jump {
                    condition: Some(_), ..
                } => TerminatorKind::Branch,
                LiftedInstruction::HandlerEntry
                | LiftedInstruction::Jump {
                    condition: None, ..
                }
                | LiftedInstruction::Subroutine { .. }
                | LiftedInstruction::SubroutineReturn(_)
                | LiftedInstruction::Nop => TerminatorKind::Goto,
                LiftedInstruction::Switch { match_value, .. } => TerminatorKind::Switch {
                    match_value: remap_operand(*match_value)?,
                },
                LiftedInstruction::Return(value) => {
                    TerminatorKind::Return(value.map(remap_operand).transpose()?)
                }
                LiftedInstruction::Throw(value) => TerminatorKind::Throw(remap_operand(*value)?),
                LiftedInstruction::Definition { .. } | LiftedInstruction::Effect(_) => {
                    if successors
                        .iter()
                        .any(|successor| matches!(successor.transfer(), ControlTransfer::Normal))
                    {
                        TerminatorKind::Fallible
                    } else {
                        TerminatorKind::Goto
                    }
                }
            };
            let terminator_id = *terminator_ids
                .get(&block.plan.id)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            if source_backed {
                source_map.insert(
                    last_location
                        .source_pc()
                        .ok_or(MokaIRBrewingError::MalformedControlFlow)?,
                    terminator_id,
                );
            }
            let terminator = Terminator::new(terminator_id, kind, successors);
            blocks.push(BasicBlock::new(
                block.plan.id,
                phis,
                instructions,
                terminator,
            ));
        }

        Ok(GeneratedMethod {
            entry,
            blocks,
            source_map,
            this_value,
            parameter_values,
            caught_exceptions,
            value_definitions,
        })
    }
}

fn allocate_instruction_id(next: &mut u32) -> Result<InstructionId, MokaIRBrewingError> {
    let id = InstructionId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
    Ok(id)
}

fn allocate_edge_id(next: &mut u32) -> Result<EdgeId, MokaIRBrewingError> {
    let id = EdgeId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
    Ok(id)
}

fn allocate_final_value(
    temp: ValueId,
    definition: ValueDefinition,
    next: &mut u32,
    values: &mut BTreeMap<ValueId, ValueId>,
    definitions: &mut Vec<ValueDefinition>,
) -> Result<ValueId, MokaIRBrewingError> {
    if values.contains_key(&temp) {
        return Err(MokaIRBrewingError::MalformedControlFlow);
    }
    let value = ValueId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
    values.insert(temp, value);
    definitions.push(definition);
    Ok(value)
}

fn resolve_final_value(
    mut value: ValueId,
    substitutions: &BTreeMap<ValueId, ValueId>,
    values: &BTreeMap<ValueId, ValueId>,
) -> Result<ValueId, MokaIRBrewingError> {
    while let Some(&replacement) = substitutions.get(&value) {
        value = replacement;
    }
    values
        .get(&value)
        .copied()
        .ok_or(MokaIRBrewingError::MalformedControlFlow)
}
