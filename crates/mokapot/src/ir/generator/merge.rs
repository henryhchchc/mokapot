use std::collections::{BTreeMap, BTreeSet};

use super::{
    MokaIRBrewingError, Operand, PairedFrameValue, ScalarBlock,
    jvm_frame::{Entry, JvmStackFrame},
    ssa,
};
use crate::ir::{BlockId, ValueId};

pub(super) fn collect_phi_candidates(
    blocks: &[ScalarBlock],
    phi_blocks: &BTreeMap<ValueId, BlockId>,
    preheader: Option<(BlockId, &JvmStackFrame<ValueId>)>,
) -> Result<ssa::PhiCandidates, MokaIRBrewingError> {
    let mut candidates = ssa::PhiCandidates::new();
    for target in blocks {
        let mut incoming = blocks
            .iter()
            .flat_map(|source| {
                source
                    .arms
                    .iter()
                    .filter(move |arm| arm.target == target.plan.id)
                    .map(move |arm| (source.plan.id, &arm.frame))
            })
            .collect::<Vec<_>>();
        if let Some((preheader_target, frame)) = &preheader
            && *preheader_target == target.plan.id
        {
            incoming.push((BlockId::new(0), frame));
        }

        let predecessor_count = incoming
            .iter()
            .map(|(predecessor, _)| *predecessor)
            .collect::<BTreeSet<_>>()
            .len();
        let mut inputs = BTreeMap::<ValueId, BTreeMap<BlockId, ValueId>>::new();
        for (predecessor, frame) in incoming {
            for (result, value) in paired_frame_values(&target.entry_frame, frame)? {
                let (Some(result), Some(value)) = (result, value) else {
                    continue;
                };
                if phi_blocks.get(&result) != Some(&target.plan.id) {
                    continue;
                }
                match inputs.entry(result).or_default().entry(predecessor) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                    std::collections::btree_map::Entry::Occupied(entry)
                        if *entry.get() == value => {}
                    std::collections::btree_map::Entry::Occupied(_) => {
                        return Err(MokaIRBrewingError::MalformedControlFlow);
                    }
                }
            }
        }
        for (&result, &block) in phi_blocks {
            if block != target.plan.id {
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
    target: &JvmStackFrame<ValueId>,
    source: &JvmStackFrame<ValueId>,
) -> Result<Vec<PairedFrameValue>, MokaIRBrewingError> {
    if target.local_variables().len() != source.local_variables().len()
        || target.operand_stack().len() != source.operand_stack().len()
    {
        return Err(MokaIRBrewingError::MalformedControlFlow);
    }
    Ok(target
        .local_variables()
        .iter()
        .zip(source.local_variables())
        .chain(target.operand_stack().iter().zip(source.operand_stack()))
        .map(|(target, source)| match (target, source) {
            (Entry::Value(result), Entry::Value(value)) => (Some(*result), Some(*value)),
            (Entry::Value(result), _) => (Some(*result), None),
            (_, Entry::Value(value)) => (None, Some(*value)),
            _ => (None, None),
        })
        .collect())
}

pub(super) fn unavailable_value_slots(
    merged: &JvmStackFrame,
    incoming: &[&JvmStackFrame],
) -> Result<(Vec<usize>, Vec<usize>), MokaIRBrewingError> {
    if incoming.iter().any(|frame| {
        frame.local_variables().len() != merged.local_variables().len()
            || frame.operand_stack().len() != merged.operand_stack().len()
    }) {
        return Err(MokaIRBrewingError::MalformedControlFlow);
    }
    let unavailable = |merged: &[Entry<Operand>], stack: bool| {
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
