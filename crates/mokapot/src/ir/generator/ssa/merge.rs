use std::collections::{BTreeMap, BTreeSet};

use super::{
    MokaIRBuildError, OperandState, ReplayedBlock, SsaFrameValue, SsaValueId,
    jvm_frame::{Entry, JvmStackFrame},
};
use crate::ir::BlockId;

type PairedFrameValue = (Option<SsaValueId>, Option<SsaValueId>);

pub(super) fn collect_phi_candidates(
    blocks: &[ReplayedBlock],
    phi_blocks: &BTreeMap<SsaValueId, BlockId>,
    preheader: Option<(BlockId, BlockId, &JvmStackFrame<SsaFrameValue>)>,
) -> Result<BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>>, MokaIRBuildError> {
    let mut candidates: BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>> = BTreeMap::new();
    for target in blocks {
        let mut incoming = blocks
            .iter()
            .flat_map(|source| {
                source
                    .arms
                    .iter()
                    .filter(move |arm| arm.target == target.id)
                    .map(move |arm| (source.id, &arm.frame))
            })
            .collect::<Vec<_>>();
        if let Some((preheader_target, preheader_id, frame)) = &preheader
            && *preheader_target == target.id
        {
            incoming.push((*preheader_id, frame));
        }

        let predecessor_count = incoming
            .iter()
            .map(|(predecessor, _)| *predecessor)
            .collect::<BTreeSet<_>>()
            .len();
        let mut inputs = BTreeMap::<SsaValueId, BTreeMap<BlockId, SsaValueId>>::new();
        for (predecessor, frame) in incoming {
            for (result, value) in paired_frame_values(&target.entry_frame, frame)? {
                let (Some(result), Some(value)) = (result, value) else {
                    continue;
                };
                if phi_blocks.get(&result) != Some(&target.id) {
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
        for (&result, &block) in phi_blocks {
            if block != target.id {
                continue;
            }
            if let Some(values) = inputs.remove(&result)
                && values.len() == predecessor_count
            {
                candidates.insert(result, values.into_iter().collect());
            }
        }
    }

    loop {
        let unavailable = phi_blocks
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
    Ok(candidates)
}

fn paired_frame_values(
    target: &JvmStackFrame<SsaFrameValue>,
    source: &JvmStackFrame<SsaFrameValue>,
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
                Entry::Value(SsaFrameValue::Value(result)),
                Entry::Value(SsaFrameValue::Value(value)),
            ) => Ok((Some(*result), Some(*value))),
            (
                Entry::Value(SsaFrameValue::ReturnAddress(lhs)),
                Entry::Value(SsaFrameValue::ReturnAddress(rhs)),
            ) if lhs == rhs => Ok((None, None)),
            (Entry::Value(SsaFrameValue::Value(result)), _) => Ok((Some(*result), None)),
            (_, Entry::Value(SsaFrameValue::Value(value))) => Ok((None, Some(*value))),
            (Entry::Value(SsaFrameValue::ReturnAddress(_)), _)
            | (_, Entry::Value(SsaFrameValue::ReturnAddress(_))) => {
                Err(MokaIRBuildError::MalformedControlFlow)
            }
            _ => Ok((None, None)),
        })
        .collect::<Result<_, _>>()
}

pub(super) fn unavailable_value_slots(
    merged: &JvmStackFrame,
    incoming: &[&JvmStackFrame],
) -> Result<(Vec<usize>, Vec<usize>), MokaIRBuildError> {
    if incoming.iter().any(|frame| {
        frame.local_variables().len() != merged.local_variables().len()
            || frame.operand_stack().len() != merged.operand_stack().len()
    }) {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }
    let unavailable = |merged: &[Entry<OperandState>], stack: bool| {
        merged
            .iter()
            .enumerate()
            .filter(|(index, entry)| {
                matches!(entry, Entry::Value(_))
                    && incoming.iter().any(|frame| {
                        let entries = if stack {
                            frame.operand_stack()
                        } else {
                            frame.local_variables()
                        };
                        !matches!(entries[*index], Entry::Value(_))
                    })
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
    };
    Ok((
        unavailable(merged.local_variables(), false),
        unavailable(merged.operand_stack(), true),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::generator::{SsaFrameValue, ssa::model::ReplayedBlock};

    #[test]
    fn preheader_uses_its_allocated_predecessor_id() {
        let descriptor = "(I)V".parse().expect("valid descriptor");
        let result = SsaValueId::new(10);
        let incoming = SsaValueId::new(11);
        let target = BlockId::new(4);
        let preheader = BlockId::new(9);
        let target_frame =
            JvmStackFrame::with_inputs(&descriptor, 1, 0, None, &[SsaFrameValue::Value(result)])
                .expect("frame fits descriptor");
        let preheader_frame =
            JvmStackFrame::with_inputs(&descriptor, 1, 0, None, &[SsaFrameValue::Value(incoming)])
                .expect("frame fits descriptor");
        let blocks = vec![ReplayedBlock {
            id: target,
            entry_frame: target_frame,
            instructions: Vec::new(),
            arms: Vec::new(),
        }];

        let candidates = collect_phi_candidates(
            &blocks,
            &BTreeMap::from([(result, target)]),
            Some((target, preheader, &preheader_frame)),
        )
        .expect("preheader provides the phi input");

        assert_eq!(candidates[&result], vec![(preheader, incoming)]);
    }
}
