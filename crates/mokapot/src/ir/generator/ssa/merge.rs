use std::collections::{BTreeMap, BTreeSet, btree_map};

use crate::ir::{
    BlockId,
    generator::{
        block_formation::{self, Merge},
        bytecode_analysis::{self, FrameMergeSite, jvm::Frame},
        error::Error,
        identity::SsaValueId,
    },
};

type PairedFrameValue = (Option<SsaValueId>, Option<SsaValueId>);

/// Provisional value and placement data for JVM frame merges.
pub(super) struct MergePlan {
    /// The value and block of every frame merge site.
    merges: BTreeMap<FrameMergeSite, Merge>,
    /// The block that computes each merge value, indexed so that phi collection
    /// never scans `merges`.
    block_by_value: BTreeMap<SsaValueId, BlockId>,
}

impl MergePlan {
    /// Plans the resolution of every frame merge.
    pub fn new(merges: BTreeMap<FrameMergeSite, Merge>) -> Self {
        let block_by_value = merges
            .values()
            .map(|merge| (merge.value, merge.block))
            .collect();
        Self {
            merges,
            block_by_value,
        }
    }

    /// The block that computes the frame merge `value`, if it is one.
    pub fn block_for(&self, value: SsaValueId) -> Option<BlockId> {
        self.block_by_value.get(&value).copied()
    }

    pub fn resolve(&self, operand: bytecode_analysis::Value) -> Result<SsaValueId, Error> {
        match operand {
            bytecode_analysis::Value::Ssa(value) => Ok(value),
            bytecode_analysis::Value::Merged(identity) => self
                .merges
                .get(&identity)
                .map(|merge| merge.value)
                .ok_or(Error::MalformedControlFlow),
            bytecode_analysis::Value::ReturnAddress(_) | bytecode_analysis::Value::Invalid => {
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
        BTreeMap::<BlockId, Vec<(BlockId, &Frame<bytecode_analysis::Value>)>>::new();
    for source in blocks {
        for arm in &source.arms {
            incoming_by_target
                .entry(arm.target)
                .or_default()
                .push((source.id, &arm.frame));
        }
    }
    let mut phis_by_block = BTreeMap::<BlockId, Vec<SsaValueId>>::new();
    for (&value, &block) in &merge_plan.block_by_value {
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
                if merge_plan.block_for(result) != Some(target.id) {
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
            .block_by_value
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
    target: &Frame<bytecode_analysis::Value>,
    source: &Frame<bytecode_analysis::Value>,
    merge_plan: &MergePlan,
) -> Result<Vec<PairedFrameValue>, Error> {
    target
        .paired_slot_values(source)
        .map_err(|_| Error::MalformedControlFlow)?
        .into_iter()
        .map(|(target, source)| match (target, source) {
            (
                Some(bytecode_analysis::Value::ReturnAddress(lhs)),
                Some(bytecode_analysis::Value::ReturnAddress(rhs)),
            ) if lhs == rhs => Ok((None, None)),
            (Some(bytecode_analysis::Value::ReturnAddress(_)), _)
            | (_, Some(bytecode_analysis::Value::ReturnAddress(_))) => {
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
            bytecode_analysis::{NodeAddress, jvm::Position},
        },
    };

    #[test]
    fn synthetic_preheader_contributes_an_ordinary_phi_input() {
        let descriptor = "(I)V".parse().expect("valid descriptor");
        let site = FrameMergeSite {
            addr: NodeAddress::entry(0.into()),
            slot: Position::Local(0),
        };
        let result = SsaValueId::new(10);
        let incoming = SsaValueId::new(11);
        let target = BlockId::new(4);
        let preheader = BlockId::new(9);
        let target_frame = Frame::for_method_entry(
            &descriptor,
            1,
            0,
            None,
            &[bytecode_analysis::Value::Ssa(result)],
        )
        .expect("frame fits descriptor");
        let preheader_frame = Frame::for_method_entry(
            &descriptor,
            1,
            0,
            None,
            &[bytecode_analysis::Value::Ssa(incoming)],
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

        let merge_plan = MergePlan::new(BTreeMap::from([(
            site,
            Merge {
                value: result,
                block: target,
            },
        )]));
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

        let merge_plan = MergePlan::new(BTreeMap::from([(
            identity,
            Merge {
                value: resolved,
                block: BlockId::new(0),
            },
        )]));
        let actual = merge_plan
            .resolve(bytecode_analysis::Value::Merged(identity))
            .expect("known merge identity resolves");

        assert_eq!(actual, resolved);
    }
}
