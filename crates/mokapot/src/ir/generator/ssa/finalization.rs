//! Lowers frame-bearing JVM blocks into scalar SSA blocks.
use super::super::block_formation::BlockEntry;
use super::{
    BTreeMap, BlockId, ControlTransfer, Instruction, MergeIdentity, MokaIRBuildError, OperandState,
    OperationKind, SsaBlock, SsaPhi, SsaSuccessor, SsaValueId, TerminatorKind,
    simplify::SimplifiedPhis,
};
use crate::ir::{
    TryMapValues,
    generator::block_formation::{JvmBlock, JvmBlockArm},
};

pub(super) fn finalize(
    entry: BlockEntry,
    blocks: Vec<JvmBlock>,
    phi_blocks: &BTreeMap<SsaValueId, BlockId>,
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
    simplified: SimplifiedPhis,
) -> Result<Vec<SsaBlock>, MokaIRBuildError> {
    // `simplify_phis` returns substitutions whose targets are already canonical.
    let canonical = |value| {
        simplified
            .substitutions
            .get(&value)
            .copied()
            .unwrap_or(value)
    };
    let mut phis_by_block = BTreeMap::<BlockId, Vec<SsaPhi>>::new();
    for (value, inputs) in simplified.candidates {
        let block = phi_blocks
            .get(&value)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let inputs = inputs
            .into_iter()
            .map(|(predecessor, value)| (predecessor, canonical(value)))
            .collect();
        phis_by_block.entry(block).or_default().push(SsaPhi {
            value: canonical(value),
            inputs,
        });
    }
    let mut finalized = Vec::with_capacity(
        blocks.len() + usize::from(matches!(entry, BlockEntry::Preheader { .. })),
    );
    if let BlockEntry::Preheader {
        synthetic,
        bytecode,
    } = entry
    {
        finalized.push(SsaBlock {
            id: synthetic,
            caught_exception: None,
            phis: vec![],
            operations: vec![],
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            successors: vec![SsaSuccessor {
                target: bytecode,
                transfer: ControlTransfer::Unconditional,
            }],
        });
    }
    for block in blocks {
        let id = block.id;
        finalized.push(finalize_block(
            block,
            phis_by_block.remove(&id).unwrap_or_default(),
            merge_values,
            &canonical,
        )?);
    }
    if phis_by_block.is_empty() {
        Ok(finalized)
    } else {
        Err(MokaIRBuildError::MalformedControlFlow)
    }
}
fn finalize_block(
    block: JvmBlock,
    phis: Vec<SsaPhi>,
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> Result<SsaBlock, MokaIRBuildError> {
    let caught_exception = block.caught_exception.map(canonical);
    let (last_location, last_instruction) = block
        .instructions
        .last()
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let terminator_source = last_instruction
        .is_explicit_transfer()
        .then(|| last_location.source_pc())
        .flatten();
    let terminator = classify_terminator(last_instruction, &block.arms, merge_values, canonical)?;
    let mut operations = Vec::new();
    for (location, instruction) in block.instructions {
        let kind = match instruction {
            Instruction::Definition { value, expr } => Some(OperationKind::Definition {
                value: canonical(value),
                expr: expr.try_map_values(|value| remap_operand(value, merge_values, canonical))?,
            }),
            Instruction::Effect(expr) => Some(OperationKind::Effect {
                expr: expr.try_map_values(|value| remap_operand(value, merge_values, canonical))?,
            }),
            _ => None,
        };
        if let Some(kind) = kind {
            operations.push((
                location
                    .source_pc()
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                kind,
            ));
        }
    }
    let successors = block
        .arms
        .into_iter()
        .map(
            |JvmBlockArm {
                 target, transfer, ..
             }| {
                Ok(SsaSuccessor {
                    target,
                    transfer: transfer
                        .try_map_values(|value| remap_operand(value, merge_values, canonical))?,
                })
            },
        )
        .collect::<Result<_, MokaIRBuildError>>()?;
    Ok(SsaBlock {
        id: block.id,
        caught_exception,
        phis,
        operations,
        terminator,
        terminator_source,
        successors,
    })
}
fn classify_terminator(
    instruction: &Instruction,
    arms: &[JvmBlockArm],
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> Result<TerminatorKind<SsaValueId>, MokaIRBuildError> {
    Ok(match instruction {
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
            match_value: remap_operand(*match_value, merge_values, canonical)?,
        },
        Instruction::Return(value) => TerminatorKind::Return(
            value
                .map(|value| remap_operand(value, merge_values, canonical))
                .transpose()?,
        ),
        Instruction::Throw(value) => {
            TerminatorKind::Throw(remap_operand(*value, merge_values, canonical)?)
        }
        Instruction::Definition { .. } | Instruction::Effect(_) => {
            if arms
                .iter()
                .any(|arm| matches!(arm.transfer, ControlTransfer::Normal))
            {
                TerminatorKind::Fallible
            } else {
                TerminatorKind::Goto
            }
        }
    })
}
fn remap_operand(
    operand: OperandState,
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> Result<SsaValueId, MokaIRBuildError> {
    match operand {
        OperandState::Value(value) => Ok(canonical(value)),
        OperandState::Merged(identity) => merge_values
            .get(&identity)
            .copied()
            .map(canonical)
            .ok_or(MokaIRBuildError::MalformedControlFlow),
        OperandState::ReturnAddress(_) | OperandState::Invalid => {
            Err(MokaIRBuildError::MalformedControlFlow)
        }
    }
}
