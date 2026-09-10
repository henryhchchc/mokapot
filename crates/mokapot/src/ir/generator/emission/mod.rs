//! Emits completed public `MokaIR` from internal SSA construction state.

mod remap;

use self::remap::{remap_expression, remap_transfer};
use super::ssa::SsaMethod;
use super::{
    BTreeMap, BasicBlock, ControlTransfer, EdgeId, InstructionId, IrOperation, LiftedInstruction,
    MokaIRBuildError, MokaIRMethod, OperationKind, Phi, PhiInput, SourceMap, SsaFrameValue,
    SsaValueId, Successor, Terminator, TerminatorKind, ValueDefinition, ValueId,
};

/// Emits final identities, blocks, and provenance from internal SSA.
pub(super) fn emit(ssa: SsaMethod<'_>) -> Result<MokaIRMethod, MokaIRBuildError> {
    Emitter { ssa }.emit()
}

struct Emitter<'method> {
    ssa: SsaMethod<'method>,
}

impl Emitter<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "final SSA allocation and block materialization form one ordered pass"
    )]
    fn emit(self) -> Result<MokaIRMethod, MokaIRBuildError> {
        let SsaMethod {
            method,
            caught_exception_ids,
            entry,
            bytecode_entry,
            needs_entry_preheader,
            blocks,
            phi_blocks,
            simplified_phis,
            this_value: this_temp,
            parameter_values,
        } = self.ssa;
        let simplified = &simplified_phis;
        let parameter_temps = &parameter_values;
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
                    u16::try_from(index).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
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
        for block in &blocks {
            let leader = *block
                .plan
                .locations
                .first()
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            if let Some(&temp) = caught_exception_ids.get(&leader) {
                let value = allocate_final_value(
                    temp,
                    ValueDefinition::CaughtException(block.plan.id),
                    &mut next_value,
                    &mut temp_values,
                    &mut value_definitions,
                )?;
                caught_exceptions.insert(block.plan.id, value);
            }

            for (&temp, &phi_block) in &phi_blocks {
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
            SsaFrameValue::Value(value) => remap(value),
            SsaFrameValue::ReturnAddress(_) => Err(MokaIRBuildError::MalformedControlFlow),
        };
        let mut source_map = SourceMap::default();
        let mut emitted_blocks =
            Vec::with_capacity(blocks.len() + usize::from(needs_entry_preheader));
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
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                TerminatorKind::Goto,
                vec![successor],
            );
            emitted_blocks.push(BasicBlock::new(entry, vec![], vec![], terminator));
        }

        for block in blocks {
            let mut phis = Vec::new();
            for (&temp, &phi_block) in &phi_blocks {
                if phi_block != block.plan.id {
                    continue;
                }
                let Some(inputs) = simplified.candidates.get(&temp) else {
                    continue;
                };
                let id = *phi_ids
                    .get(&temp)
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?;
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
                        Some(OperationKind::Definition {
                            value: remap(value)?,
                            expr: remap_expression(expr, &remap_operand)?,
                        })
                    }
                    LiftedInstruction::Effect(expr) => Some(OperationKind::Effect {
                        expr: remap_expression(expr, &remap_operand)?,
                    }),
                    LiftedInstruction::HandlerEntry
                    | LiftedInstruction::Unwind
                    | LiftedInstruction::Erased
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
                        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
                    let pc = location
                        .source_pc()
                        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
                    source_map.insert(pc, id);
                    instructions.push(IrOperation::new(id, kind));
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
                .collect::<Result<Vec<_>, MokaIRBuildError>>()?;
            let (last_location, last) = block
                .instructions
                .last()
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
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
                | LiftedInstruction::Erased => TerminatorKind::Goto,
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
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            if source_backed {
                source_map.insert(
                    last_location
                        .source_pc()
                        .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                    terminator_id,
                );
            }
            let terminator = Terminator::new(terminator_id, kind, successors);
            emitted_blocks.push(BasicBlock::new(
                block.plan.id,
                phis,
                instructions,
                terminator,
            ));
        }

        Ok(MokaIRMethod::new(
            method.access_flags,
            method.name.clone(),
            method.descriptor.clone(),
            method.owner.clone(),
            entry,
            emitted_blocks,
            source_map,
            this_value,
            parameter_values,
            caught_exceptions,
            value_definitions,
        ))
    }
}

fn allocate_instruction_id(next: &mut u32) -> Result<InstructionId, MokaIRBuildError> {
    let id = InstructionId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    Ok(id)
}

fn allocate_edge_id(next: &mut u32) -> Result<EdgeId, MokaIRBuildError> {
    let id = EdgeId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    Ok(id)
}

fn allocate_final_value(
    temp: SsaValueId,
    definition: ValueDefinition,
    next: &mut u32,
    values: &mut BTreeMap<SsaValueId, ValueId>,
    definitions: &mut Vec<ValueDefinition>,
) -> Result<ValueId, MokaIRBuildError> {
    if values.contains_key(&temp) {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }
    let value = ValueId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    values.insert(temp, value);
    definitions.push(definition);
    Ok(value)
}

fn resolve_final_value(
    mut value: SsaValueId,
    substitutions: &BTreeMap<SsaValueId, SsaValueId>,
    values: &BTreeMap<SsaValueId, ValueId>,
) -> Result<ValueId, MokaIRBuildError> {
    while let Some(&replacement) = substitutions.get(&value) {
        value = replacement;
    }
    values
        .get(&value)
        .copied()
        .ok_or(MokaIRBuildError::MalformedControlFlow)
}
