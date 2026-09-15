use std::collections::{BTreeMap, BTreeSet, btree_map};

use crate::ir::{
    BlockId,
    generator::{
        block_formation,
        error::Error,
        identity::SsaValueId,
        jvm::{
            frame::Frame,
            symbolic_execution::{self, FrameMergeSite},
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

    pub fn resolve(&self, operand: symbolic_execution::Value) -> Result<SsaValueId, Error> {
        match operand {
            symbolic_execution::Value::Ssa(value) => Ok(value),
            symbolic_execution::Value::Merged(identity) => self
                .merge_values
                .get(&identity)
                .copied()
                .ok_or(Error::MalformedControlFlow),
            symbolic_execution::Value::ReturnAddress(_) | symbolic_execution::Value::Invalid => {
                Err(Error::MalformedControlFlow)
            }
        }
    }
}

pub(super) fn collect_phi_candidates(
    blocks: &[block_formation::Block],
    merge_plan: &MergePlan,
) -> Result<BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>>, Error> {
    let mut candidates: BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>> = BTreeMap::new();
    let mut incoming_by_target =
        BTreeMap::<BlockId, Vec<(BlockId, &Frame<symbolic_execution::Value>)>>::new();
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
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                    btree_map::Entry::Occupied(entry) if *entry.get() == value => {}
                    btree_map::Entry::Occupied(_) => {
                        return Err(Error::MalformedControlFlow);
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
        return Err(Error::MalformedControlFlow);
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
    target: &Frame<symbolic_execution::Value>,
    source: &Frame<symbolic_execution::Value>,
    merge_plan: &MergePlan,
) -> Result<Vec<PairedFrameValue>, Error> {
    target
        .paired_slot_values(source)
        .map_err(|_| Error::MalformedControlFlow)?
        .into_iter()
        .map(|(target, source)| match (target, source) {
            (
                Some(symbolic_execution::Value::ReturnAddress(lhs)),
                Some(symbolic_execution::Value::ReturnAddress(rhs)),
            ) if lhs == rhs => Ok((None, None)),
            (Some(symbolic_execution::Value::ReturnAddress(_)), _)
            | (_, Some(symbolic_execution::Value::ReturnAddress(_))) => {
                Err(Error::MalformedControlFlow)
            }
            (Some(result), Some(value)) => Ok((
                Some(merge_plan.resolve(*result)?),
                Some(merge_plan.resolve(*value)?),
            )),
            (Some(result), None) => Ok((Some(merge_plan.resolve(*result)?), None)),
            (None, Some(value)) => Ok((None, Some(merge_plan.resolve(*value)?))),
            (None, None) => Ok((None, None)),
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
            block_formation,
            jvm::{frame::Position, symbolic_execution::NodeAddress},
        },
    };

    #[test]
    fn synthetic_preheader_contributes_an_ordinary_phi_input() {
        let descriptor = "(I)V".parse().expect("valid descriptor");
        let result = SsaValueId::new(10);
        let incoming = SsaValueId::new(11);
        let target = BlockId::new(4);
        let preheader = BlockId::new(9);
        let target_frame = Frame::for_method_entry(
            &descriptor,
            1,
            0,
            None,
            &[symbolic_execution::Value::Ssa(result)],
        )
        .expect("frame fits descriptor");
        let preheader_frame = Frame::for_method_entry(
            &descriptor,
            1,
            0,
            None,
            &[symbolic_execution::Value::Ssa(incoming)],
        )
        .expect("frame fits descriptor");
        let blocks = vec![
            block_formation::Block {
                id: preheader,
                entry_frame: preheader_frame.clone(),
                operations: Vec::new(),
                terminator: TerminatorKind::Goto,
                terminator_source: None,
                arms: vec![block_formation::Arm {
                    target,
                    transfer: ControlTransfer::Unconditional,
                    frame: preheader_frame,
                }],
                caught_exception: None,
            },
            block_formation::Block {
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
            addr: NodeAddress::Unwind,
            slot: Position::Local(0),
        };
        let resolved = SsaValueId::new(12);

        let merge_plan = MergePlan::new(BTreeMap::from([(identity, resolved)]), BTreeMap::new());
        let actual = merge_plan
            .resolve(symbolic_execution::Value::Merged(identity))
            .expect("known merge identity resolves");

        assert_eq!(actual, resolved);
    }
}
