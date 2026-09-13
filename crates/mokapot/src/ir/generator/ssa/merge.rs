use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    BlockId,
    generator::{
        block_formation::JvmBlock,
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            frame::{Entry, JvmStackFrame},
            symbolic_execution::{FrameMergeSite, SymbolicValue},
        },
    },
};

type PairedFrameValue = (Option<SsaValueId>, Option<SsaValueId>);

/// Provisional value and placement data for JVM frame merges.
pub(super) struct MergePlan {
    merge_values: BTreeMap<FrameMergeSite, SsaValueId>,
    phi_blocks: BTreeMap<SsaValueId, BlockId>,
}

impl MergePlan {
    pub const fn new(
        merge_values: BTreeMap<FrameMergeSite, SsaValueId>,
        phi_blocks: BTreeMap<SsaValueId, BlockId>,
    ) -> Self {
        Self {
            merge_values,
            phi_blocks,
        }
    }

    pub fn block_for(&self, value: SsaValueId) -> Option<BlockId> {
        self.phi_blocks.get(&value).copied()
    }

    pub fn resolve(&self, operand: SymbolicValue) -> Result<SsaValueId, MokaIRBuildError> {
        match operand {
            SymbolicValue::Value(value) => Ok(value),
            SymbolicValue::Merged(identity) => self
                .merge_values
                .get(&identity)
                .copied()
                .ok_or(MokaIRBuildError::MalformedControlFlow),
            SymbolicValue::ReturnAddress(_) | SymbolicValue::Invalid => {
                Err(MokaIRBuildError::MalformedControlFlow)
            }
        }
    }
}

pub(super) fn collect_phi_candidates(
    blocks: &[JvmBlock],
    merge_plan: &MergePlan,
) -> Result<BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>>, MokaIRBuildError> {
    let mut candidates: BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>> = BTreeMap::new();
    let mut incoming_by_target =
        BTreeMap::<BlockId, Vec<(BlockId, &JvmStackFrame<SymbolicValue>)>>::new();
    for source in blocks {
        for arm in &source.arms {
            incoming_by_target
                .entry(arm.target)
                .or_default()
                .push((source.id, &arm.frame));
        }
    }
    let mut phis_by_block = BTreeMap::<BlockId, Vec<SsaValueId>>::new();
    for (&value, &block) in &merge_plan.phi_blocks {
        phis_by_block.entry(block).or_default().push(value);
    }

    for target in blocks {
        let incoming = incoming_by_target.remove(&target.id).unwrap_or_default();
        let predecessor_count = incoming
            .iter()
            .map(|(predecessor, _)| *predecessor)
            .collect::<BTreeSet<_>>()
            .len();
        let mut inputs = BTreeMap::<SsaValueId, BTreeMap<BlockId, SsaValueId>>::new();
        for (predecessor, frame) in incoming {
            for (result, value) in paired_frame_values(&target.entry_frame, frame, merge_plan)? {
                let (Some(result), Some(value)) = (result, value) else {
                    continue;
                };
                if merge_plan.phi_blocks.get(&result) != Some(&target.id) {
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
        let unavailable = merge_plan
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
    Ok(candidates)
}

fn paired_frame_values(
    target: &JvmStackFrame<SymbolicValue>,
    source: &JvmStackFrame<SymbolicValue>,
    merge_plan: &MergePlan,
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
                Entry::Value(SymbolicValue::ReturnAddress(lhs)),
                Entry::Value(SymbolicValue::ReturnAddress(rhs)),
            ) if lhs == rhs => Ok((None, None)),
            (Entry::Value(SymbolicValue::ReturnAddress(_)), _)
            | (_, Entry::Value(SymbolicValue::ReturnAddress(_))) => {
                Err(MokaIRBuildError::MalformedControlFlow)
            }
            (Entry::Value(result), Entry::Value(value)) => Ok((
                Some(merge_plan.resolve(*result)?),
                Some(merge_plan.resolve(*value)?),
            )),
            (Entry::Value(result), _) => Ok((Some(merge_plan.resolve(*result)?), None)),
            (_, Entry::Value(value)) => Ok((None, Some(merge_plan.resolve(*value)?))),
            _ => Ok((None, None)),
        })
        .collect::<Result<_, _>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            block_formation::{JvmBlock, JvmBlockArm},
            jvm::{frame::FrameSlot, normalization::Location},
        },
    };

    #[test]
    fn synthetic_preheader_contributes_an_ordinary_phi_input() {
        let descriptor = "(I)V".parse().expect("valid descriptor");
        let result = SsaValueId::new(10);
        let incoming = SsaValueId::new(11);
        let target = BlockId::new(4);
        let preheader = BlockId::new(9);
        let target_frame =
            JvmStackFrame::with_inputs(&descriptor, 1, 0, None, &[SymbolicValue::Value(result)])
                .expect("frame fits descriptor");
        let preheader_frame =
            JvmStackFrame::with_inputs(&descriptor, 1, 0, None, &[SymbolicValue::Value(incoming)])
                .expect("frame fits descriptor");
        let blocks = vec![
            JvmBlock {
                id: preheader,
                entry_frame: preheader_frame.clone(),
                operations: Vec::new(),
                terminator: TerminatorKind::Goto,
                terminator_source: None,
                arms: vec![JvmBlockArm {
                    target,
                    transfer: ControlTransfer::Unconditional,
                    frame: preheader_frame,
                }],
                caught_exception: None,
            },
            JvmBlock {
                id: target,
                entry_frame: target_frame,
                operations: Vec::new(),
                terminator: TerminatorKind::Goto,
                terminator_source: None,
                arms: Vec::new(),
                caught_exception: None,
            },
        ];

        let merge_plan = MergePlan::new(BTreeMap::new(), BTreeMap::from([(result, target)]));
        let candidates =
            collect_phi_candidates(&blocks, &merge_plan).expect("preheader provides the phi input");

        assert_eq!(candidates[&result], vec![(preheader, incoming)]);
    }

    #[test]
    fn merge_plan_resolves_merge_identities() {
        let identity = FrameMergeSite {
            location: Location::Unwind,
            slot: FrameSlot::Local(0),
        };
        let resolved = SsaValueId::new(12);

        let merge_plan = MergePlan::new(BTreeMap::from([(identity, resolved)]), BTreeMap::new());
        let actual = merge_plan
            .resolve(SymbolicValue::Merged(identity))
            .expect("known merge identity resolves");

        assert_eq!(actual, resolved);
    }
}
