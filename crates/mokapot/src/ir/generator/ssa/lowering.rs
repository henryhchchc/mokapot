//! Lowers the frame-rich block graph into provisional scalar SSA.

use std::collections::{BTreeMap, BTreeSet, btree_map};

use crate::ir::{
    BlockId, TryMapValues,
    generator::{
        block_formation::{self, FrameMerge},
        bytecode_analysis::{self, FrameMergeSite, jvm::Frame},
        error::Error,
        identity::SsaValueId,
        ssa::model::{PhiCandidate, ScalarBlock, Successor},
    },
};

struct PairedFrameValue {
    result: Option<SsaValueId>,
    input: Option<SsaValueId>,
}

/// The frame-free result of lowering formed JVM blocks.
pub(super) struct LoweredGraph {
    pub(super) entry: BlockId,
    pub(super) blocks: Vec<ScalarBlock>,
    pub(super) phi_candidates: BTreeMap<SsaValueId, PhiCandidate>,
    pub(super) this_value: Option<SsaValueId>,
    pub(super) parameter_values: Vec<SsaValueId>,
}

/// Validated indexes used only while crossing the JVM-frame boundary.
struct MergeCatalog {
    value_by_site: BTreeMap<FrameMergeSite, SsaValueId>,
    block_by_value: BTreeMap<SsaValueId, BlockId>,
}

impl MergeCatalog {
    fn new(
        merges: BTreeMap<FrameMergeSite, FrameMerge>,
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
        for (site, FrameMerge { value, block }) in merges {
            if !known_blocks.contains(&block) || block_by_value.insert(value, block).is_some() {
                return Err(Error::MalformedControlFlow);
            }
            value_by_site.insert(site, value);
        }
        Ok(Self {
            value_by_site,
            block_by_value,
        })
    }

    fn block_for(&self, value: SsaValueId) -> Option<BlockId> {
        self.block_by_value.get(&value).copied()
    }

    fn placements(&self) -> impl Iterator<Item = (SsaValueId, BlockId)> + '_ {
        self.block_by_value
            .iter()
            .map(|(&value, &block)| (value, block))
    }

    fn resolve(&self, operand: bytecode_analysis::Value) -> Result<SsaValueId, Error> {
        match operand {
            bytecode_analysis::Value::Ssa(value) => Ok(value),
            bytecode_analysis::Value::Merged(site) => self
                .value_by_site
                .get(&site)
                .copied()
                .ok_or(Error::MalformedControlFlow),
            bytecode_analysis::Value::ReturnAddress(_) | bytecode_analysis::Value::Invalid => {
                Err(Error::MalformedControlFlow)
            }
        }
    }
}

/// Consumes the frame-rich graph and resolves every surviving operand to a
/// scalar identity before returning.
pub(super) fn lower(graph: block_formation::BlockGraph) -> Result<LoweredGraph, Error> {
    let block_formation::BlockGraph {
        entry,
        blocks,
        merges,
        this_value,
        parameter_values,
    } = graph;
    let merge_catalog = MergeCatalog::new(merges, blocks.iter().map(|block| block.id))?;
    let phi_candidates = collect_phi_candidates(&blocks, &merge_catalog)?;
    let blocks = blocks
        .into_iter()
        .map(|block| lower_block(block, &merge_catalog))
        .collect::<Result<_, _>>()?;

    Ok(LoweredGraph {
        entry,
        blocks,
        phi_candidates,
        this_value,
        parameter_values,
    })
}

fn lower_block(
    block: block_formation::Block,
    merge_catalog: &MergeCatalog,
) -> Result<ScalarBlock, Error> {
    let block_formation::Block {
        id,
        entry_frame: _,
        operations,
        end,
        caught_exception,
    } = block;
    let (terminator, terminator_source, arms) = end.into_parts();
    let resolve = |operand| merge_catalog.resolve(operand);
    let operations = operations
        .into_iter()
        .map(|(source, operation)| {
            operation
                .try_map_values(&resolve)
                .map(|operation| (source, operation))
        })
        .collect::<Result<_, _>>()?;
    let terminator = terminator.try_map_values(&resolve)?;
    let successors = arms
        .into_iter()
        .map(|arm| {
            let transfer = arm.transfer.try_map_values(&resolve)?;
            Ok(Successor {
                target: arm.target,
                transfer,
            })
        })
        .collect::<Result<_, Error>>()?;

    Ok(ScalarBlock {
        id,
        caught_exception,
        operations,
        terminator,
        terminator_source,
        successors,
    })
}

fn collect_phi_candidates(
    blocks: &[block_formation::Block],
    merge_catalog: &MergeCatalog,
) -> Result<BTreeMap<SsaValueId, PhiCandidate>, Error> {
    let mut candidates = BTreeMap::new();
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
            for PairedFrameValue { result, input } in
                paired_frame_values(&target.entry_frame, frame, merge_catalog)?
            {
                let (Some(result), Some(value)) = (result, input) else {
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
                candidates.insert(
                    result,
                    PhiCandidate {
                        placement: target.id,
                        inputs: values.into_iter().collect(),
                    },
                );
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
            .filter(|(_, candidate)| {
                candidate
                    .inputs
                    .iter()
                    .any(|(_, value)| unavailable.contains(value))
            })
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
            ) if lhs == rhs => Ok(PairedFrameValue {
                result: None,
                input: None,
            }),
            (Some(bytecode_analysis::Value::ReturnAddress(_)), _)
            | (_, Some(bytecode_analysis::Value::ReturnAddress(_))) => {
                Err(Error::MalformedControlFlow)
            }
            (Some(result), Some(value)) => Ok(PairedFrameValue {
                result: Some(merge_catalog.resolve(*result)?),
                input: Some(merge_catalog.resolve(*value)?),
            }),
            (Some(result), None) => Ok(PairedFrameValue {
                result: Some(merge_catalog.resolve(*result)?),
                input: None,
            }),
            (None, Some(value)) => Ok(PairedFrameValue {
                result: None,
                input: Some(merge_catalog.resolve(*value)?),
            }),
            (None, None) => Ok(PairedFrameValue {
                result: None,
                input: None,
            }),
        })
        .collect::<Result<_, _>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        expression::{Expression, MathOperation},
        generator::bytecode_analysis::{NodeAddress, jvm::Position, jvm::ValueCategory},
    };

    fn local_frame(value: bytecode_analysis::Value) -> Frame<bytecode_analysis::Value> {
        Frame::for_method_entry(
            &"(I)V".parse().expect("valid descriptor"),
            1,
            0,
            None,
            &[value],
        )
        .expect("frame fits descriptor")
    }

    fn merge_site() -> FrameMergeSite {
        FrameMergeSite {
            addr: NodeAddress::entry(0.into()),
            slot: Position::Local(0),
        }
    }

    fn empty_frame(max_locals: u16) -> Frame<bytecode_analysis::Value> {
        Frame::for_method_entry(
            &"()V".parse().expect("valid descriptor"),
            max_locals,
            0,
            None,
            &[],
        )
        .expect("frame fits descriptor")
    }

    #[test]
    fn lowering_returns_only_scalar_blocks_and_placed_candidates() {
        let site = merge_site();
        let result = SsaValueId::new(10);
        let incoming = SsaValueId::new(11);
        let operation_input = SsaValueId::new(12);
        let target = BlockId::new(1);
        let preheader = BlockId::new(0);
        let preheader_frame = local_frame(bytecode_analysis::Value::Ssa(incoming));
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
                entry_frame: local_frame(bytecode_analysis::Value::Merged(site)),
                operations: vec![(
                    4.into(),
                    OperationKind::Definition {
                        value: bytecode_analysis::Value::Merged(site),
                        expr: Expression::Math(MathOperation::Negate(
                            bytecode_analysis::Value::Ssa(operation_input),
                        )),
                    },
                )],
                end: block_formation::BlockEnd::new(
                    TerminatorKind::Return(Some(bytecode_analysis::Value::Merged(site))),
                    Some(5.into()),
                    Vec::new(),
                )
                .expect("valid return end"),
                caught_exception: Some(SsaValueId::new(13)),
            },
        ];
        let graph = block_formation::BlockGraph {
            entry: preheader,
            blocks,
            merges: BTreeMap::from([(
                site,
                FrameMerge {
                    value: result,
                    block: target,
                },
            )]),
            this_value: None,
            parameter_values: vec![incoming],
        };

        let lowered = lower(graph).expect("valid graph lowers");
        let candidate = &lowered.phi_candidates[&result];
        assert_eq!(candidate.placement, target);
        assert_eq!(candidate.inputs, [(preheader, incoming)]);
        let _: &[(BlockId, SsaValueId)] = &candidate.inputs;
        let _: &ControlTransfer<SsaValueId> = &lowered.blocks[0].successors[0].transfer;
        let target = &lowered.blocks[1];
        let _: &OperationKind<SsaValueId> = &target.operations[0].1;
        let _: &TerminatorKind<SsaValueId> = &target.terminator;
        assert_eq!(target.caught_exception, Some(SsaValueId::new(13)));
        assert!(matches!(
            target.operations[0].1,
            OperationKind::Definition {
                value,
                expr: Expression::Math(MathOperation::Negate(input)),
            } if value == result && input == operation_input
        ));
        assert_eq!(target.terminator, TerminatorKind::Return(Some(result)));
    }

    #[test]
    fn rejects_conflicting_parallel_inputs_from_one_predecessor() {
        let site = merge_site();
        let result = SsaValueId::new(10);
        let target = BlockId::new(1);
        let source = BlockId::new(0);
        let source_frame = local_frame(bytecode_analysis::Value::Ssa(SsaValueId::new(11)));
        let blocks = vec![
            block_formation::Block {
                id: source,
                entry_frame: source_frame,
                operations: Vec::new(),
                end: block_formation::BlockEnd::new(
                    TerminatorKind::Throw(bytecode_analysis::Value::Ssa(SsaValueId::new(20))),
                    None,
                    vec![
                        block_formation::Arm {
                            target,
                            transfer: ControlTransfer::Exception(None),
                            frame: local_frame(bytecode_analysis::Value::Ssa(SsaValueId::new(11))),
                        },
                        block_formation::Arm {
                            target,
                            transfer: ControlTransfer::Exception(None),
                            frame: local_frame(bytecode_analysis::Value::Ssa(SsaValueId::new(12))),
                        },
                    ],
                )
                .expect("valid parallel exceptional arms"),
                caught_exception: None,
            },
            block_formation::Block {
                id: target,
                entry_frame: local_frame(bytecode_analysis::Value::Merged(site)),
                operations: Vec::new(),
                end: block_formation::BlockEnd::new(TerminatorKind::Unwind, None, Vec::new())
                    .expect("valid unwind end"),
                caught_exception: None,
            },
        ];
        let graph = block_formation::BlockGraph {
            entry: source,
            blocks,
            merges: BTreeMap::from([(
                site,
                FrameMerge {
                    value: result,
                    block: target,
                },
            )]),
            this_value: None,
            parameter_values: Vec::new(),
        };

        assert!(matches!(lower(graph), Err(Error::MalformedControlFlow)));
    }

    #[test]
    fn prunes_candidates_that_transitively_depend_on_a_missing_input() {
        let first_site = merge_site();
        let second_site = FrameMergeSite {
            addr: NodeAddress::entry(0.into()),
            slot: Position::Local(1),
        };
        let first = SsaValueId::new(10);
        let second = SsaValueId::new(11);
        let source = BlockId::new(0);
        let target = BlockId::new(1);
        let mut source_frame = empty_frame(2);
        source_frame
            .locals
            .set(
                1,
                bytecode_analysis::Value::Merged(first_site),
                ValueCategory::Category1,
            )
            .expect("local is in range");
        let mut target_frame = empty_frame(2);
        target_frame
            .locals
            .set(
                0,
                bytecode_analysis::Value::Merged(first_site),
                ValueCategory::Category1,
            )
            .expect("local is in range");
        target_frame
            .locals
            .set(
                1,
                bytecode_analysis::Value::Merged(second_site),
                ValueCategory::Category1,
            )
            .expect("local is in range");
        let graph = block_formation::BlockGraph {
            entry: source,
            blocks: vec![
                block_formation::Block {
                    id: source,
                    entry_frame: source_frame.clone(),
                    operations: Vec::new(),
                    end: block_formation::BlockEnd::new(
                        TerminatorKind::Goto,
                        None,
                        vec![block_formation::Arm {
                            target,
                            transfer: ControlTransfer::Unconditional,
                            frame: source_frame,
                        }],
                    )
                    .expect("valid goto end"),
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
            ],
            merges: BTreeMap::from([
                (
                    first_site,
                    FrameMerge {
                        value: first,
                        block: target,
                    },
                ),
                (
                    second_site,
                    FrameMerge {
                        value: second,
                        block: target,
                    },
                ),
            ]),
            this_value: None,
            parameter_values: Vec::new(),
        };

        let lowered = lower(graph).expect("missing inputs are pruned");

        assert!(lowered.phi_candidates.is_empty());
    }

    #[test]
    fn rejects_duplicate_provisional_values() {
        let value = SsaValueId::new(12);
        let block = BlockId::new(0);
        let merges = BTreeMap::from([
            (merge_site(), FrameMerge { value, block }),
            (
                FrameMergeSite {
                    addr: NodeAddress::Unwind,
                    slot: Position::Local(0),
                },
                FrameMerge { value, block },
            ),
        ]);

        assert!(matches!(
            MergeCatalog::new(merges, [block]),
            Err(Error::MalformedControlFlow)
        ));
    }

    #[test]
    fn rejects_missing_merge_placement() {
        let merges = BTreeMap::from([(
            merge_site(),
            FrameMerge {
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
    fn rejects_duplicate_block_identities() {
        let block = BlockId::new(0);

        assert!(matches!(
            MergeCatalog::new(BTreeMap::new(), [block, block]),
            Err(Error::MalformedControlFlow)
        ));
    }

    #[test]
    fn rejects_unknown_merge_operands() {
        let catalog =
            MergeCatalog::new(BTreeMap::new(), [BlockId::new(0)]).expect("empty catalog is valid");

        assert!(matches!(
            catalog.resolve(bytecode_analysis::Value::Merged(merge_site())),
            Err(Error::MalformedControlFlow)
        ));
    }
}
