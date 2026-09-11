//! Lowers replay state into scalar SSA blocks.
use super::super::block_formation::BlockEntry;
use super::{
    BTreeMap, BlockId, ControlTransfer, Instruction, JvmReplayPlan, MokaIRBuildError,
    OperationKind, ReplayedArm, ReplayedBlock, SsaBlock, SsaFrameValue, SsaPhi, SsaSuccessor,
    SsaValueId, TerminatorKind, simplify::SimplifiedPhis,
};
use crate::ir::TryMapValues;

pub(super) fn finalize(
    entry: BlockEntry,
    replay: &JvmReplayPlan,
    blocks: Vec<ReplayedBlock>,
    phi_blocks: &BTreeMap<SsaValueId, BlockId>,
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
            replay,
            phis_by_block.remove(&id).unwrap_or_default(),
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
    block: ReplayedBlock,
    replay: &JvmReplayPlan,
    phis: Vec<SsaPhi>,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> Result<SsaBlock, MokaIRBuildError> {
    let caught_exception = block
        .instructions
        .first()
        .and_then(|(location, _)| replay.caught_exception(*location))
        .map(canonical);
    let (last_location, last_instruction) = block
        .instructions
        .last()
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let terminator_source = last_instruction
        .is_explicit_transfer()
        .then(|| last_location.source_pc())
        .flatten();
    let terminator = classify_terminator(last_instruction, &block.arms, canonical)?;
    let mut operations = Vec::new();
    for (location, instruction) in block.instructions {
        let kind = match instruction {
            Instruction::Definition { value, expr } => Some(OperationKind::Definition {
                value: canonical(value),
                expr: expr.try_map_values(|value| remap_operand(value, canonical))?,
            }),
            Instruction::Effect(expr) => Some(OperationKind::Effect {
                expr: expr.try_map_values(|value| remap_operand(value, canonical))?,
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
            |ReplayedArm {
                 target, transfer, ..
             }| {
                Ok(SsaSuccessor {
                    target,
                    transfer: transfer.try_map_values(|value| remap_operand(value, canonical))?,
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
    instruction: &Instruction<SsaFrameValue>,
    arms: &[ReplayedArm],
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
            match_value: remap_operand(*match_value, canonical)?,
        },
        Instruction::Return(value) => TerminatorKind::Return(
            value
                .map(|value| remap_operand(value, canonical))
                .transpose()?,
        ),
        Instruction::Throw(value) => TerminatorKind::Throw(remap_operand(*value, canonical)?),
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
    operand: SsaFrameValue,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> Result<SsaValueId, MokaIRBuildError> {
    match operand {
        SsaFrameValue::Value(value) => Ok(canonical(value)),
        SsaFrameValue::ReturnAddress(_) => Err(MokaIRBuildError::MalformedControlFlow),
    }
}
