//! Forms semantic maximal blocks from analyzed JVM locations and frame facts.

mod jvm_block;

pub(in crate::ir::generator) use jvm_block::{JvmBlock, JvmBlockArm};

use super::{
    AnalyzedJvmCfg, BTreeMap, BTreeSet, BlockId, ControlTransfer, Instruction, Location,
    MergeIdentity, MokaIRBuildError, OperandState, OperationKind, SsaValueId, TerminatorKind,
};

/// Block-level JVM graph consumed by SSA construction.
pub(super) struct JvmBlockGraph {
    pub entry: BlockId,
    pub blocks: Vec<JvmBlock>,
    pub phi_blocks: BTreeMap<SsaValueId, BlockId>,
    pub merge_values: BTreeMap<MergeIdentity, SsaValueId>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Forms maximal semantic blocks from completed JVM frame facts.
#[expect(
    clippy::too_many_lines,
    reason = "block partitioning and identity allocation form one invariant-preserving pass"
)]
pub(super) fn form(analyzed_cfg: AnalyzedJvmCfg) -> Result<JvmBlockGraph, MokaIRBuildError> {
    let entry_location = analyzed_cfg.entry_location;
    let reachable = analyzed_cfg.locations.keys().copied().collect::<Vec<_>>();
    if reachable.is_empty() {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }

    let mut leaders = BTreeSet::from([entry_location]);
    leaders.extend(
        analyzed_cfg
            .phi_values
            .keys()
            .map(|identity| identity.location),
    );
    leaders.extend(
        reachable
            .iter()
            .copied()
            .filter(|location| !matches!(location, Location::Bytecode { .. })),
    );
    let mut predecessors: BTreeMap<Location, BTreeSet<Location>> = BTreeMap::new();
    for (&source, facts) in &analyzed_cfg.locations {
        for outgoing in &facts.outgoing {
            predecessors
                .entry(outgoing.target)
                .or_default()
                .insert(source);
        }
    }
    leaders.extend(
        predecessors
            .iter()
            .filter(|(_, sources)| sources.len() > 1)
            .map(|(target, _)| *target),
    );

    for facts in analyzed_cfg.locations.values() {
        if facts.instruction.is_explicit_transfer()
            || facts.outgoing.iter().any(|outgoing| {
                matches!(
                    outgoing.transfer,
                    ControlTransfer::Normal
                        | ControlTransfer::Exception(_)
                        | ControlTransfer::Unwind
                )
            })
        {
            leaders.extend(facts.outgoing.iter().map(|outgoing| outgoing.target));
        }
    }

    for pair in reachable.windows(2) {
        let [current, next] = pair else {
            unreachable!()
        };
        let facts = analyzed_cfg
            .locations
            .get(current)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let plain_fallthrough = !facts.instruction.is_explicit_transfer()
            && facts.outgoing.len() == 1
            && facts.outgoing[0].target == *next
            && matches!(facts.outgoing[0].transfer, ControlTransfer::Unconditional);
        if !plain_fallthrough {
            leaders.insert(*next);
        }
    }
    leaders.retain(|location| analyzed_cfg.locations.contains_key(location));

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
                .ok_or(MokaIRBuildError::MalformedControlFlow)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let bytecode_entry = *block_ids
        .get(&entry_location)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
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
        let id = current_block.ok_or(MokaIRBuildError::MalformedControlFlow)?;
        location_to_block.insert(location, id);
        grouped.entry(id).or_default().push(location);
    }
    let phi_blocks = analyzed_cfg
        .phi_values
        .iter()
        .map(|(identity, &value)| {
            location_to_block
                .get(&identity.location)
                .copied()
                .map(|block| (value, block))
                .ok_or(MokaIRBuildError::MalformedControlFlow)
        })
        .collect::<Result<_, _>>()?;
    let mut analyzed_locations = analyzed_cfg.locations;
    let mut blocks = grouped
        .into_iter()
        .map(|(id, locations)| {
            let mut entry_frame = None;
            let mut caught_exception = None;
            let mut operations = Vec::with_capacity(locations.len());
            let mut terminator = None;
            let mut terminator_source = None;
            let mut arms = Vec::new();
            for (index, location) in locations.iter().copied().enumerate() {
                let facts = analyzed_locations
                    .remove(&location)
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?;
                if index == 0 {
                    entry_frame = Some(facts.incoming);
                    caught_exception = facts.caught_exception;
                }
                let is_last = index + 1 == locations.len();
                if is_last {
                    arms = facts
                        .outgoing
                        .into_iter()
                        .map(|outgoing| {
                            location_to_block
                                .get(&outgoing.target)
                                .copied()
                                .map(|target| JvmBlockArm {
                                    target,
                                    transfer: outgoing.transfer,
                                    frame: outgoing.frame,
                                })
                                .ok_or(MokaIRBuildError::MalformedControlFlow)
                        })
                        .collect::<Result<_, _>>()?;
                } else {
                    let next = locations[index + 1];
                    if facts.instruction.is_explicit_transfer()
                        || facts.outgoing.len() != 1
                        || facts.outgoing[0].target != next
                        || !matches!(facts.outgoing[0].transfer, ControlTransfer::Unconditional)
                    {
                        return Err(MokaIRBuildError::MalformedControlFlow);
                    }
                }
                if is_last {
                    let explicit_transfer = facts.instruction.is_explicit_transfer();
                    let has_normal_successor = arms
                        .iter()
                        .any(|arm| matches!(arm.transfer, ControlTransfer::Normal));
                    let (operation, kind) =
                        classify_block_end(facts.instruction, has_normal_successor);
                    if let Some(operation) = operation {
                        operations.push((
                            location
                                .source_pc()
                                .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                            operation,
                        ));
                    }
                    terminator = Some(kind);
                    terminator_source = explicit_transfer.then(|| location.source_pc()).flatten();
                } else {
                    match facts.instruction {
                        Instruction::Definition { value, expr } => operations.push((
                            location
                                .source_pc()
                                .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                            OperationKind::Definition {
                                value: OperandState::Value(value),
                                expr,
                            },
                        )),
                        Instruction::Effect(expr) => operations.push((
                            location
                                .source_pc()
                                .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                            OperationKind::Effect { expr },
                        )),
                        Instruction::Erased => {}
                        Instruction::HandlerEntry
                        | Instruction::Unwind
                        | Instruction::Jump { .. }
                        | Instruction::Switch { .. }
                        | Instruction::Return(_)
                        | Instruction::Throw(_)
                        | Instruction::Subroutine { .. }
                        | Instruction::SubroutineReturn(_) => {
                            return Err(MokaIRBuildError::MalformedControlFlow);
                        }
                    }
                }
            }
            Ok(JvmBlock {
                id,
                entry_frame: entry_frame.ok_or(MokaIRBuildError::MalformedControlFlow)?,
                operations,
                terminator: terminator.ok_or(MokaIRBuildError::MalformedControlFlow)?,
                terminator_source,
                arms,
                caught_exception,
            })
        })
        .collect::<Result<Vec<_>, MokaIRBuildError>>()?;
    if !analyzed_locations.is_empty() {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }

    if needs_entry_preheader {
        blocks.insert(
            0,
            JvmBlock {
                id: entry,
                entry_frame: analyzed_cfg.initial_frame.clone(),
                operations: Vec::new(),
                terminator: TerminatorKind::Goto,
                terminator_source: None,
                arms: vec![JvmBlockArm {
                    target: bytecode_entry,
                    transfer: ControlTransfer::Unconditional,
                    frame: analyzed_cfg.initial_frame,
                }],
                caught_exception: None,
            },
        );
    }

    Ok(JvmBlockGraph {
        entry,
        blocks,
        phi_blocks,
        merge_values: analyzed_cfg.phi_values,
        this_value: analyzed_cfg.this_value,
        parameter_values: analyzed_cfg.parameter_values,
    })
}

fn classify_block_end(
    instruction: Instruction,
    has_normal_successor: bool,
) -> (
    Option<OperationKind<OperandState>>,
    TerminatorKind<OperandState>,
) {
    match instruction {
        Instruction::Unwind => (None, TerminatorKind::Unwind),
        Instruction::Jump {
            condition: Some(_), ..
        } => (None, TerminatorKind::Branch),
        Instruction::HandlerEntry
        | Instruction::Jump {
            condition: None, ..
        }
        | Instruction::Subroutine { .. }
        | Instruction::SubroutineReturn(_)
        | Instruction::Erased => (None, TerminatorKind::Goto),
        Instruction::Switch { match_value, .. } => (None, TerminatorKind::Switch { match_value }),
        Instruction::Return(value) => (None, TerminatorKind::Return(value)),
        Instruction::Throw(value) => (None, TerminatorKind::Throw(value)),
        Instruction::Definition { value, expr } => (
            Some(OperationKind::Definition {
                value: OperandState::Value(value),
                expr,
            }),
            implicit_terminator(has_normal_successor),
        ),
        Instruction::Effect(expr) => (
            Some(OperationKind::Effect { expr }),
            implicit_terminator(has_normal_successor),
        ),
    }
}

const fn implicit_terminator(has_normal_successor: bool) -> TerminatorKind<OperandState> {
    if has_normal_successor {
        TerminatorKind::Fallible
    } else {
        TerminatorKind::Goto
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ir::{expression::Expression, generator::ReturnAddress},
        jvm::ConstantValue,
    };

    #[test]
    fn pseudo_and_legacy_instructions_become_semantic_gotos() {
        let instructions = [
            Instruction::HandlerEntry,
            Instruction::Erased,
            Instruction::Subroutine {
                target: Location::Unwind,
            },
            Instruction::SubroutineReturn(OperandState::ReturnAddress(ReturnAddress::for_test(0))),
        ];

        for instruction in instructions {
            let (operation, terminator) = classify_block_end(instruction, false);
            assert!(operation.is_none());
            assert_eq!(terminator, TerminatorKind::Goto);
        }
    }

    #[test]
    fn fallible_definition_becomes_an_operation_and_terminator() {
        let value = SsaValueId::new(7);
        let (operation, terminator) = classify_block_end(
            Instruction::Definition {
                value,
                expr: Expression::Const(ConstantValue::Integer(1)),
            },
            true,
        );

        assert!(matches!(
            operation,
            Some(OperationKind::Definition {
                value: OperandState::Value(actual),
                ..
            }) if actual == value
        ));
        assert_eq!(terminator, TerminatorKind::Fallible);
    }
}
