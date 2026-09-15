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

/// Validated bidirectional indexes for JVM frame merges.
pub(super) struct MergeCatalog {
    /// The provisional value for every frame merge site.
    value_by_site: BTreeMap<FrameMergeSite, SsaValueId>,
    /// The block that computes each provisional merge value.
    block_by_value: BTreeMap<SsaValueId, BlockId>,
}

impl MergeCatalog {
    /// Indexes merges after checking that their values and placements are unambiguous.
    pub fn new(
        merges: BTreeMap<FrameMergeSite, Merge>,
        known_blocks: impl IntoIterator<Item = BlockId>,
    ) -> Result<Self, Error> {
        let known_blocks =
            known_blocks
                .into_iter()
                .try_fold(BTreeSet::new(), |mut known_blocks, block| {
                    if !known_blocks.insert(block) {
                        return Err(Error::MalformedControlFlow);
                    }
                    Ok(known_blocks)
                })?;
        let mut value_by_site = BTreeMap::new();
        let mut block_by_value = BTreeMap::new();
        for (site, Merge { value, block }) in merges {
            if !known_blocks.contains(&block) {
                return Err(Error::MalformedControlFlow);
            }
            value_by_site.insert(site, value);
            if block_by_value.insert(value, block).is_some() {
                return Err(Error::MalformedControlFlow);
            }
        }
        Ok(Self {
            value_by_site,
            block_by_value,
        })
    }

    /// The block that computes the frame merge `value`, if it is one.
    pub fn block_for(&self, value: SsaValueId) -> Option<BlockId> {
        self.block_by_value.get(&value).copied()
    }

    /// Iterates over every provisional merge value and its computing block.
    fn placements(&self) -> impl Iterator<Item = (SsaValueId, BlockId)> + '_ {
        self.block_by_value
            .iter()
            .map(|(&value, &block)| (value, block))
    }

    pub fn resolve(&self, operand: bytecode_analysis::Value) -> Result<SsaValueId, Error> {
        match operand {
            bytecode_analysis::Value::Ssa(value) => Ok(value),
            bytecode_analysis::Value::Merged(identity) => self
                .value_by_site
                .get(&identity)
                .copied()
                .ok_or(Error::MalformedControlFlow),
            bytecode_analysis::Value::ReturnAddress(_) | bytecode_analysis::Value::Invalid => {
                Err(Error::MalformedControlFlow)
            }
        }
    }
}

pub(super) fn collect_phi_candidates(
    blocks: &[block_formation::Block],
    merge_catalog: &MergeCatalog,
) -> Result<BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>>, Error> {
    let mut candidates: BTreeMap<SsaValueId, Vec<(BlockId, SsaValueId)>> = BTreeMap::new();
    let mut incoming_by_target =
        BTreeMap::<BlockId, Vec<(BlockId, &Frame<bytecode_analysis::Value>)>>::new();
    for source in blocks {
        for arm in source.end.arms() {
            incoming_by_target
                .entry(arm.target)
                .or_default()
                .push((source.id, &arm.frame));
        }
    }
    let mut phis_by_block = BTreeMap::<BlockId, Vec<SsaValueId>>::new();
    for (value, block) in merge_catalog.placements() {
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
            for (result, value) in paired_frame_values(&target.entry_frame, frame, merge_catalog)? {
                let (Some(result), Some(value)) = (result, value) else {
                    continue;
                };
                if merge_catalog.block_for(result) != Some(target.id) {
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
        let unavailable = merge_catalog
            .placements()
            .map(|(value, _)| value)
            .filter(|result| !candidates.contains_key(result))
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
    merge_catalog: &MergeCatalog,
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
                Some(merge_catalog.resolve(*result)?),
                Some(merge_catalog.resolve(*value)?),
            )),
            (Some(result), None) => Ok((Some(merge_catalog.resolve(*result)?), None)),
            (None, Some(value)) => Ok((None, Some(merge_catalog.resolve(*value)?))),
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
                end: block_formation::BlockEnd::new(
                    TerminatorKind::Goto,
                    None,
                    vec![block_formation::Arm {
                        target,
                        transfer: ControlTransfer::Unconditional,
                        frame: preheader_frame,
                    }],
                )
                .expect("valid preheader end"),
                caught_exception: None,
            },
            block_formation::Block {
                id: target,
                entry_frame: target_frame,
                operations: Vec::new(),
                end: block_formation::BlockEnd::new(TerminatorKind::Unwind, None, Vec::new())
                    .expect("valid unwind end"),
                caught_exception: None,
            },
        ];

        let merge_catalog = MergeCatalog::new(
            BTreeMap::from([(
                site,
                Merge {
                    value: result,
                    block: target,
                },
            )]),
            [preheader, target],
        )
        .expect("valid merge catalog");
        let candidates = collect_phi_candidates(&blocks, &merge_catalog)
            .expect("preheader provides the phi input");

        assert_eq!(candidates[&result], vec![(preheader, incoming)]);
    }

    #[test]
    fn merge_catalog_resolves_merge_identities_and_looks_up_placements() {
        let identity = FrameMergeSite {
            addr: NodeAddress::Unwind,
            slot: Position::Local(0),
        };
        let resolved = SsaValueId::new(12);

        let block = BlockId::new(0);
        let merge_catalog = MergeCatalog::new(
            BTreeMap::from([(
                identity,
                Merge {
                    value: resolved,
                    block,
                },
            )]),
            [block],
        )
        .expect("valid merge catalog");
        let actual = merge_catalog
            .resolve(bytecode_analysis::Value::Merged(identity))
            .expect("known merge identity resolves");

        assert_eq!(actual, resolved);
        assert_eq!(merge_catalog.block_for(resolved), Some(block));
    }

    #[test]
    fn merge_catalog_rejects_duplicate_provisional_values() {
        let value = SsaValueId::new(12);
        let block = BlockId::new(0);
        let merges = BTreeMap::from([
            (
                FrameMergeSite {
                    addr: NodeAddress::entry(0.into()),
                    slot: Position::Local(0),
                },
                Merge { value, block },
            ),
            (
                FrameMergeSite {
                    addr: NodeAddress::Unwind,
                    slot: Position::Local(0),
                },
                Merge { value, block },
            ),
        ]);

        assert!(matches!(
            MergeCatalog::new(merges, [block]),
            Err(Error::MalformedControlFlow)
        ));
    }

    #[test]
    fn merge_catalog_rejects_missing_placement() {
        let merges = BTreeMap::from([(
            FrameMergeSite {
                addr: NodeAddress::Unwind,
                slot: Position::Local(0),
            },
            Merge {
                value: SsaValueId::new(12),
                block: BlockId::new(1),
            },
        )]);

        assert!(matches!(
            MergeCatalog::new(merges, [BlockId::new(0)]),
            Err(Error::MalformedControlFlow)
        ));
    }

    #[test]
    fn merge_catalog_rejects_duplicate_blocks() {
        let block = BlockId::new(0);

        assert!(matches!(
            MergeCatalog::new(BTreeMap::new(), [block, block]),
            Err(Error::MalformedControlFlow)
        ));
    }

    #[test]
    fn resolving_unknown_merge_site_is_malformed() {
        let known = FrameMergeSite {
            addr: NodeAddress::Unwind,
            slot: Position::Local(0),
        };
        let unknown = FrameMergeSite {
            addr: NodeAddress::entry(0.into()),
            slot: Position::Local(0),
        };
        let merge_catalog = MergeCatalog::new(
            BTreeMap::from([(
                known,
                Merge {
                    value: SsaValueId::new(12),
                    block: BlockId::new(0),
                },
            )]),
            [BlockId::new(0)],
        )
        .expect("valid merge catalog");

        assert!(matches!(
            merge_catalog.resolve(bytecode_analysis::Value::Merged(unknown)),
            Err(Error::MalformedControlFlow)
        ));
    }
}
