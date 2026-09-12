use std::collections::{BTreeMap, BTreeSet};

use super::{
    Instruction, MergeIdentity, MokaIRBuildError, OperandState, SsaValueId,
    model::{LoweredBlock, LoweredBlockArm, LoweredOperand},
};
use crate::ir::generator::{
    block_formation::{JvmBlock, JvmBlockArm},
    jvm_frame::{Entry, JvmStackFrame},
};
use crate::ir::{BlockId, TryMapValues};

type PairedFrameValue = (Option<SsaValueId>, Option<SsaValueId>);

/// Lookup data needed while collecting phis from JVM frames.
pub(super) struct PhiPlan {
    phi_blocks: BTreeMap<SsaValueId, BlockId>,
    merge_values: BTreeMap<MergeIdentity, SsaValueId>,
}

impl PhiPlan {
    pub(super) const fn new(
        phi_blocks: BTreeMap<SsaValueId, BlockId>,
        merge_values: BTreeMap<MergeIdentity, SsaValueId>,
    ) -> Self {
        Self {
            phi_blocks,
            merge_values,
        }
    }
}

/// The phi placement data needed after frame-dependent lowering has completed.
pub(super) struct FinalPhiPlan {
    phi_blocks: BTreeMap<SsaValueId, BlockId>,
}

impl FinalPhiPlan {
    pub(super) fn block_for(&self, value: SsaValueId) -> Option<BlockId> {
        self.phi_blocks.get(&value).copied()
    }
}

/// Phi candidates and the blocks whose JVM frame state has been discarded.
pub(super) struct PhiCollection {
    pub candidates: BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>>,
    pub blocks: Vec<LoweredBlock>,
    pub phi_plan: FinalPhiPlan,
}

pub(super) fn collect_phi_candidates(
    blocks: Vec<JvmBlock>,
    phi_plan: PhiPlan,
    preheader: Option<(BlockId, BlockId, &JvmStackFrame)>,
) -> Result<PhiCollection, MokaIRBuildError> {
    let mut candidates: BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>> = BTreeMap::new();
    let mut incoming_by_target = BTreeMap::<BlockId, Vec<(BlockId, &JvmStackFrame)>>::new();
    for source in &blocks {
        for arm in &source.arms {
            incoming_by_target
                .entry(arm.target)
                .or_default()
                .push((source.id, &arm.frame));
        }
    }
    if let Some((target, predecessor, frame)) = preheader {
        incoming_by_target
            .entry(target)
            .or_default()
            .push((predecessor, frame));
    }
    let mut phis_by_block = BTreeMap::<BlockId, Vec<SsaValueId>>::new();
    for (&value, &block) in &phi_plan.phi_blocks {
        phis_by_block.entry(block).or_default().push(value);
    }

    for target in &blocks {
        let incoming = incoming_by_target.remove(&target.id).unwrap_or_default();
        let predecessor_count = incoming
            .iter()
            .map(|(predecessor, _)| *predecessor)
            .collect::<BTreeSet<_>>()
            .len();
        let mut inputs = BTreeMap::<SsaValueId, BTreeMap<BlockId, SsaValueId>>::new();
        for (predecessor, frame) in incoming {
            for (result, value) in
                paired_frame_values(&target.entry_frame, frame, &phi_plan.merge_values)?
            {
                let (Some(result), Some(value)) = (result, value) else {
                    continue;
                };
                if phi_plan.phi_blocks.get(&result) != Some(&target.id) {
                    continue;
                }
                match inputs.entry(result).or_default().entry(predecessor) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                    std::collections::btree_map::Entry::Occupied(entry)
                        if *entry.get() == value => {}
                    std::collections::btree_map::Entry::Occupied(_) => {
                        return Err(MokaIRBuildError::MalformedControlFlow);
                    }
                }
            }
        }
        for result in phis_by_block.remove(&target.id).unwrap_or_default() {
            if let Some(values) = inputs.remove(&result)
                && values.len() == predecessor_count
            {
                candidates.insert(result, values.into_iter().collect());
            }
        }
    }
    if !incoming_by_target.is_empty() || !phis_by_block.is_empty() {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }

    loop {
        let unavailable = phi_plan
            .phi_blocks
            .keys()
            .filter(|result| !candidates.contains_key(result))
            .copied()
            .collect::<BTreeSet<_>>();
        let invalid = candidates
            .iter()
            .filter(|(_, inputs)| inputs.iter().any(|(_, value)| unavailable.contains(value)))
            .map(|(result, _)| *result)
            .collect::<Vec<_>>();
        if invalid.is_empty() {
            break;
        }
        for result in invalid {
            candidates.remove(&result);
        }
    }
    let PhiPlan {
        phi_blocks,
        merge_values,
    } = phi_plan;
    let blocks = lower_blocks(blocks, &merge_values)?;
    Ok(PhiCollection {
        candidates,
        blocks,
        phi_plan: FinalPhiPlan { phi_blocks },
    })
}

fn lower_blocks(
    blocks: Vec<JvmBlock>,
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
) -> Result<Vec<LoweredBlock>, MokaIRBuildError> {
    blocks
        .into_iter()
        .map(
            |JvmBlock {
                 id,
                 entry_frame: _,
                 instructions,
                 arms,
                 caught_exception,
             }| {
                let instructions = instructions
                    .into_iter()
                    .map(|(location, instruction)| {
                        lower_instruction(instruction, merge_values)
                            .map(|instruction| (location, instruction))
                    })
                    .collect::<Result<_, _>>()?;
                let arms = arms
                    .into_iter()
                    .map(
                        |JvmBlockArm {
                             target,
                             transfer,
                             frame: _,
                         }| {
                            transfer
                                .try_map_values(|value| lower_operand(value, merge_values))
                                .map(|transfer| LoweredBlockArm { target, transfer })
                        },
                    )
                    .collect::<Result<_, _>>()?;
                Ok(LoweredBlock {
                    id,
                    instructions,
                    arms,
                    caught_exception,
                })
            },
        )
        .collect()
}

fn lower_instruction(
    instruction: Instruction,
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
) -> Result<Instruction<LoweredOperand>, MokaIRBuildError> {
    Ok(match instruction {
        Instruction::HandlerEntry => Instruction::HandlerEntry,
        Instruction::Unwind => Instruction::Unwind,
        Instruction::Erased => Instruction::Erased,
        Instruction::Definition { value, expr } => Instruction::Definition {
            value,
            expr: expr.try_map_values(|value| lower_operand(value, merge_values))?,
        },
        Instruction::Effect(expr) => {
            Instruction::Effect(expr.try_map_values(|value| lower_operand(value, merge_values))?)
        }
        Instruction::Jump { condition, target } => Instruction::Jump {
            condition: condition
                .map(|condition| {
                    condition.try_map_values(|value| lower_operand(value, merge_values))
                })
                .transpose()?,
            target,
        },
        Instruction::Switch {
            match_value,
            branches,
            default,
        } => Instruction::Switch {
            match_value: lower_operand(match_value, merge_values)?,
            branches,
            default,
        },
        Instruction::Return(value) => Instruction::Return(
            value
                .map(|value| lower_operand(value, merge_values))
                .transpose()?,
        ),
        Instruction::Throw(value) => Instruction::Throw(lower_operand(value, merge_values)?),
        Instruction::Subroutine { target } => Instruction::Subroutine { target },
        Instruction::SubroutineReturn(value) => {
            Instruction::SubroutineReturn(lower_operand(value, merge_values)?)
        }
    })
}

fn lower_operand(
    operand: OperandState,
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
) -> Result<LoweredOperand, MokaIRBuildError> {
    match operand {
        OperandState::Value(value) => Ok(LoweredOperand::Value(value)),
        OperandState::Merged(identity) => merge_values
            .get(&identity)
            .copied()
            .map(LoweredOperand::Value)
            .ok_or(MokaIRBuildError::MalformedControlFlow),
        OperandState::ReturnAddress(address) => Ok(LoweredOperand::ReturnAddress(address)),
        OperandState::Invalid => Err(MokaIRBuildError::MalformedControlFlow),
    }
}

fn paired_frame_values(
    target: &JvmStackFrame,
    source: &JvmStackFrame,
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
) -> Result<Vec<PairedFrameValue>, MokaIRBuildError> {
    if target.local_variables().len() != source.local_variables().len()
        || target.operand_stack().len() != source.operand_stack().len()
    {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }
    target
        .local_variables()
        .iter()
        .zip(source.local_variables())
        .chain(target.operand_stack().iter().zip(source.operand_stack()))
        .map(|(target, source)| match (target, source) {
            (
                Entry::Value(OperandState::ReturnAddress(lhs)),
                Entry::Value(OperandState::ReturnAddress(rhs)),
            ) if lhs == rhs => Ok((None, None)),
            (Entry::Value(OperandState::ReturnAddress(_)), _)
            | (_, Entry::Value(OperandState::ReturnAddress(_))) => {
                Err(MokaIRBuildError::MalformedControlFlow)
            }
            (Entry::Value(result), Entry::Value(value)) => Ok((
                Some(value_id(*result, merge_values)?),
                Some(value_id(*value, merge_values)?),
            )),
            (Entry::Value(result), _) => Ok((Some(value_id(*result, merge_values)?), None)),
            (_, Entry::Value(value)) => Ok((None, Some(value_id(*value, merge_values)?))),
            _ => Ok((None, None)),
        })
        .collect::<Result<_, _>>()
}

fn value_id(
    value: OperandState,
    merge_values: &BTreeMap<MergeIdentity, SsaValueId>,
) -> Result<SsaValueId, MokaIRBuildError> {
    match value {
        OperandState::Value(value) => Ok(value),
        OperandState::Merged(identity) => merge_values
            .get(&identity)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow),
        OperandState::ReturnAddress(_) | OperandState::Invalid => {
            Err(MokaIRBuildError::MalformedControlFlow)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::generator::{Location, block_formation::JvmBlock, jvm_frame::FrameSlot};

    #[test]
    fn preheader_uses_its_allocated_predecessor_id() {
        let descriptor = "(I)V".parse().expect("valid descriptor");
        let result = SsaValueId::new(10);
        let incoming = SsaValueId::new(11);
        let target = BlockId::new(4);
        let preheader = BlockId::new(9);
        let target_frame =
            JvmStackFrame::with_inputs(&descriptor, 1, 0, None, &[OperandState::Value(result)])
                .expect("frame fits descriptor");
        let preheader_frame =
            JvmStackFrame::with_inputs(&descriptor, 1, 0, None, &[OperandState::Value(incoming)])
                .expect("frame fits descriptor");
        let blocks = vec![JvmBlock {
            id: target,
            entry_frame: target_frame,
            instructions: Vec::new(),
            arms: Vec::new(),
            caught_exception: None,
        }];

        let collected = collect_phi_candidates(
            blocks,
            PhiPlan::new(BTreeMap::from([(result, target)]), BTreeMap::new()),
            Some((target, preheader, &preheader_frame)),
        )
        .expect("preheader provides the phi input");

        assert_eq!(collected.candidates[&result], vec![(preheader, incoming)]);
    }

    #[test]
    fn lowering_resolves_merge_identities_before_finalization() {
        let identity = MergeIdentity {
            location: Location::Unwind,
            slot: FrameSlot::Local(0),
        };
        let resolved = SsaValueId::new(12);

        let instruction = lower_instruction(
            Instruction::Return(Some(OperandState::Merged(identity))),
            &BTreeMap::from([(identity, resolved)]),
        )
        .expect("known merge identity resolves");

        assert!(
            matches!(instruction, Instruction::Return(Some(LoweredOperand::Value(value))) if value == resolved)
        );
    }
}
