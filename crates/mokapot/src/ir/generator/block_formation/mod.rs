//! Forms maximal basic blocks from analyzed JVM locations.

mod jvm_block;

pub(in crate::ir::generator) use jvm_block::JvmBlock;

use super::{
    AnalyzedJvmCfg, BTreeMap, BTreeSet, BlockId, JvmReplayPlan, JvmStackFrame, JvmTransferCategory,
    Location, MokaIRBuildError,
};

/// How control enters the formed block graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ir::generator) enum BlockEntry {
    /// The method begins directly at the bytecode entry block.
    Direct(BlockId),
    /// A synthetic block separates method entry from a loop header.
    Preheader {
        synthetic: BlockId,
        bytecode: BlockId,
    },
}

impl BlockEntry {
    pub(in crate::ir::generator) const fn method_entry(&self) -> BlockId {
        match *self {
            Self::Direct(block)
            | Self::Preheader {
                synthetic: block, ..
            } => block,
        }
    }

    pub(in crate::ir::generator) const fn bytecode_entry(&self) -> BlockId {
        match *self {
            Self::Direct(block)
            | Self::Preheader {
                bytecode: block, ..
            } => block,
        }
    }
}

/// Block-level JVM graph consumed by SSA construction.
pub(super) struct JvmBlockGraph {
    pub entry: BlockEntry,
    pub initial_frame: JvmStackFrame,
    pub blocks: Vec<JvmBlock>,
    pub location_to_block: BTreeMap<Location, BlockId>,
    pub replay: JvmReplayPlan,
}

/// Forms maximal basic blocks from completed JVM frame facts.
#[expect(
    clippy::too_many_lines,
    reason = "block partitioning and identity allocation form one invariant-preserving pass"
)]
pub(super) fn form(analyzed_cfg: AnalyzedJvmCfg) -> Result<JvmBlockGraph, MokaIRBuildError> {
    let entry_location = analyzed_cfg.entry_location;
    let reachable = analyzed_cfg.locations.keys().copied().collect::<Vec<_>>();
    if reachable.is_empty() {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }

    let mut leaders = BTreeSet::from([entry_location]);
    leaders.extend(
        reachable
            .iter()
            .copied()
            .filter(|location| !matches!(location, Location::Bytecode { .. })),
    );
    let mut predecessors: BTreeMap<Location, BTreeSet<Location>> = BTreeMap::new();
    let mut incoming_frames: BTreeMap<Location, Vec<JvmStackFrame>> = BTreeMap::new();
    for (&source, facts) in &analyzed_cfg.locations {
        for outgoing in &facts.outgoing {
            predecessors
                .entry(outgoing.target)
                .or_default()
                .insert(source);
            incoming_frames
                .entry(outgoing.target)
                .or_default()
                .push(outgoing.frame.clone());
        }
    }
    leaders.extend(
        predecessors
            .iter()
            .filter(|(_, sources)| sources.len() > 1)
            .map(|(target, _)| *target),
    );

    for facts in analyzed_cfg.locations.values() {
        if facts.is_explicit_transfer
            || facts.outgoing.iter().any(|outgoing| {
                matches!(
                    outgoing.transfer,
                    JvmTransferCategory::Normal
                        | JvmTransferCategory::Exception
                        | JvmTransferCategory::Unwind
                )
            })
        {
            leaders.extend(facts.outgoing.iter().map(|outgoing| outgoing.target));
        }
    }

    for pair in reachable.windows(2) {
        let [current, next] = pair else {
            unreachable!()
        };
        let facts = analyzed_cfg
            .locations
            .get(current)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let plain_fallthrough = !facts.is_explicit_transfer
            && facts.outgoing.len() == 1
            && facts.outgoing[0].target == *next
            && matches!(
                facts.outgoing[0].transfer,
                JvmTransferCategory::Unconditional
            );
        if !plain_fallthrough {
            leaders.insert(*next);
        }
    }
    leaders.retain(|location| analyzed_cfg.locations.contains_key(location));

    let needs_entry_preheader = predecessors
        .get(&entry_location)
        .is_some_and(|sources| !sources.is_empty());
    let block_offset = u32::from(needs_entry_preheader);
    let block_ids = leaders
        .iter()
        .enumerate()
        .map(|(index, location)| {
            u32::try_from(index)
                .ok()
                .and_then(|index| index.checked_add(block_offset))
                .map(|index| (*location, BlockId::new(index)))
                .ok_or(MokaIRBuildError::MalformedControlFlow)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let bytecode_entry = *block_ids
        .get(&entry_location)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let entry = if needs_entry_preheader {
        BlockEntry::Preheader {
            synthetic: BlockId::new(0),
            bytecode: bytecode_entry,
        }
    } else {
        BlockEntry::Direct(bytecode_entry)
    };

    let mut location_to_block = BTreeMap::new();
    let mut grouped: BTreeMap<BlockId, Vec<Location>> = BTreeMap::new();
    let mut current_block = None;
    for location in reachable {
        if let Some(id) = block_ids.get(&location) {
            current_block = Some(*id);
        }
        let id = current_block.ok_or(MokaIRBuildError::MalformedControlFlow)?;
        location_to_block.insert(location, id);
        grouped.entry(id).or_default().push(location);
    }
    let blocks = grouped
        .into_iter()
        .map(|(id, locations)| {
            let leader = *locations
                .first()
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            let analyzed_entry = analyzed_cfg
                .locations
                .get(&leader)
                .map(|facts| facts.incoming.clone())
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            Ok(JvmBlock {
                id,
                locations,
                analyzed_entry,
                incoming_frames: incoming_frames.remove(&leader).unwrap_or_default(),
            })
        })
        .collect::<Result<Vec<_>, MokaIRBuildError>>()?;

    Ok(JvmBlockGraph {
        entry,
        initial_frame: analyzed_cfg.initial_frame,
        blocks,
        location_to_block,
        replay: analyzed_cfg.replay,
    })
}
