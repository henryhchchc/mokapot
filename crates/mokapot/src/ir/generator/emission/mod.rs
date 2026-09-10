//! Emits completed public `MokaIR` from internal SSA construction state.

mod remap;

use self::remap::{remap_expression, remap_transfer};
use super::block_formation::BlockEntry;
use super::ssa::SsaGraph;
use super::{
    BTreeMap, BasicBlock, ControlTransfer, EdgeId, Instruction, InstructionId, MokaIRBuildError,
    MokaIRMethod, Operation, OperationKind, Phi, PhiInput, SourceMap, SsaFrameValue, SsaValueId,
    Successor, Terminator, TerminatorKind, ValueDefinition, ValueId,
};

/// Emits final identities, blocks, and provenance from internal SSA.
#[expect(
    clippy::too_many_lines,
    reason = "final SSA allocation and block materialization form one ordered pass"
)]
pub(super) fn emit(
    method: &super::Method,
    ssa: SsaGraph,
) -> Result<MokaIRMethod, MokaIRBuildError> {
    let SsaGraph {
        caught_exceptions: caught_exception_temps,
        entry,
        blocks,
        phis: ssa_phis,
        value_aliases,
        this_value: this_temp,
        parameter_values,
    } = ssa;
    let parameter_temps = &parameter_values;
    let mut next_instruction = 0_u32;
    let mut next_value = 0_u32;
    let mut temp_values = BTreeMap::new();
    let mut value_definitions = Vec::new();
    let mut phi_ids = BTreeMap::new();
    let mut instruction_ids = BTreeMap::new();
    let mut terminator_ids = BTreeMap::new();
    let method_entry = entry.method_entry();

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
            let index = u16::try_from(index).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
            allocate_final_value(
                temp,
                ValueDefinition::Parameter(index),
                &mut next_value,
                &mut temp_values,
                &mut value_definitions,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    if matches!(entry, BlockEntry::Preheader { .. }) {
        terminator_ids.insert(
            method_entry,
            allocate_instruction_id(&mut next_instruction)?,
        );
    }

    let mut caught_exceptions = BTreeMap::new();
    for block in &blocks {
        if let Some(&temp) = caught_exception_temps.get(&block.id) {
            let value = allocate_final_value(
                temp,
                ValueDefinition::CaughtException(block.id),
                &mut next_value,
                &mut temp_values,
                &mut value_definitions,
            )?;
            caught_exceptions.insert(block.id, value);
        }

        for phi in ssa_phis.iter().filter(|phi| phi.block == block.id) {
            let id = allocate_instruction_id(&mut next_instruction)?;
            phi_ids.insert(phi.value, id);
            allocate_final_value(
                phi.value,
                ValueDefinition::Instruction(id),
                &mut next_value,
                &mut temp_values,
                &mut value_definitions,
            )?;
        }

        for (location, instruction) in &block.instructions {
            let retained = matches!(
                instruction,
                Instruction::Definition { .. } | Instruction::Effect(_)
            );
            if !retained {
                continue;
            }
            let id = allocate_instruction_id(&mut next_instruction)?;
            instruction_ids.insert(*location, id);
            if let Instruction::Definition { value, .. } = instruction {
                allocate_final_value(
                    *value,
                    ValueDefinition::Instruction(id),
                    &mut next_value,
                    &mut temp_values,
                    &mut value_definitions,
                )?;
            }
        }
        terminator_ids.insert(block.id, allocate_instruction_id(&mut next_instruction)?);
    }

    let remap = |value| resolve_final_value(value, &value_aliases, &temp_values);
    let remap_operand = |value| match value {
        SsaFrameValue::Value(value) => remap(value),
        SsaFrameValue::ReturnAddress(_) => Err(MokaIRBuildError::MalformedControlFlow),
    };
    let mut source_map = SourceMap::default();
    let mut emitted_blocks = Vec::with_capacity(
        blocks.len() + usize::from(matches!(entry, BlockEntry::Preheader { .. })),
    );
    let mut next_edge = 0_u32;
    if let BlockEntry::Preheader {
        synthetic,
        bytecode,
    } = entry
    {
        let successor = Successor::new(
            allocate_edge_id(&mut next_edge)?,
            bytecode,
            ControlTransfer::Unconditional,
        );
        let terminator = Terminator::new(
            *terminator_ids
                .get(&synthetic)
                .ok_or(MokaIRBuildError::MalformedControlFlow)?,
            TerminatorKind::Goto,
            vec![successor],
        );
        emitted_blocks.push(BasicBlock::new(synthetic, vec![], vec![], terminator));
    }

    for block in blocks {
        let mut phis = Vec::new();
        for phi in ssa_phis.iter().filter(|phi| phi.block == block.id) {
            let id = *phi_ids
                .get(&phi.value)
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            let value = remap(phi.value)?;
            let inputs = phi
                .inputs
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
                Instruction::Definition { value, expr } => Some(OperationKind::Definition {
                    value: remap(value)?,
                    expr: remap_expression(expr, &remap_operand)?,
                }),
                Instruction::Effect(expr) => Some(OperationKind::Effect {
                    expr: remap_expression(expr, &remap_operand)?,
                }),
                Instruction::HandlerEntry
                | Instruction::Unwind
                | Instruction::Erased
                | Instruction::Subroutine { .. }
                | Instruction::Jump { .. }
                | Instruction::Switch { .. }
                | Instruction::Return(_)
                | Instruction::Throw(_)
                | Instruction::SubroutineReturn(_) => None,
            };
            if let Some(kind) = kind {
                let id = *instruction_ids
                    .get(location)
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?;
                let pc = location
                    .source_pc()
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?;
                source_map.insert(pc, id);
                instructions.push(Operation::new(id, kind));
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
            Instruction::Unwind => TerminatorKind::Unwind,
            Instruction::Jump {
                condition: Some(_), ..
            } => TerminatorKind::Branch,
            Instruction::HandlerEntry
            | Instruction::Jump {
                condition: None, ..
            }
            | Instruction::Subroutine { .. }
            | Instruction::SubroutineReturn(_)
            | Instruction::Erased => TerminatorKind::Goto,
            Instruction::Switch { match_value, .. } => TerminatorKind::Switch {
                match_value: remap_operand(*match_value)?,
            },
            Instruction::Return(value) => {
                TerminatorKind::Return(value.map(remap_operand).transpose()?)
            }
            Instruction::Throw(value) => TerminatorKind::Throw(remap_operand(*value)?),
            Instruction::Definition { .. } | Instruction::Effect(_) => {
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
            .get(&block.id)
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
        emitted_blocks.push(BasicBlock::new(block.id, phis, instructions, terminator));
    }

    Ok(MokaIRMethod::new(
        method.access_flags,
        method.name.clone(),
        method.descriptor.clone(),
        method.owner.clone(),
        method_entry,
        emitted_blocks,
        source_map,
        this_value,
        parameter_values,
        caught_exceptions,
        value_definitions,
    ))
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
