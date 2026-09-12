//! Discovers the structural layout of semantic JVM blocks.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    BlockId,
    control_flow::ControlTransfer,
    generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            analysis::{AnalyzedJvmCfg, AnalyzedLocation, MergeIdentity},
            normalization::Location,
        },
    },
};

/// The partition of analyzed locations into semantic blocks.
pub(super) struct BlockLayout {
    entry: BlockId,
    bytecode_entry: BlockId,
    location_to_block: BTreeMap<Location, BlockId>,
    locations_by_block: BTreeMap<BlockId, Vec<Location>>,
}

impl BlockLayout {
    pub fn discover(analyzed_cfg: &AnalyzedJvmCfg) -> Result<Self, MokaIRBuildError> {
        let reachable = analyzed_cfg.locations.keys().copied().collect::<Vec<_>>();
        if reachable.is_empty() {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }

        let predecessors = predecessor_locations(&analyzed_cfg.locations);
        let leaders = discover_leaders(analyzed_cfg, &reachable, &predecessors)?;
        let needs_entry_preheader = predecessors
            .get(&analyzed_cfg.entry_location)
            .is_some_and(|sources| !sources.is_empty());
        let (entry, bytecode_entry, block_ids) =
            allocate_block_ids(&leaders, analyzed_cfg.entry_location, needs_entry_preheader)?;
        let grouped = group_locations(&reachable, &block_ids)?;

        Ok(Self {
            entry,
            bytecode_entry,
            location_to_block: grouped.block_by_location,
            locations_by_block: grouped.locations_by_block,
        })
    }

    pub const fn entry(&self) -> BlockId {
        self.entry
    }

    pub const fn locations(&self) -> &BTreeMap<BlockId, Vec<Location>> {
        &self.locations_by_block
    }

    pub fn block_at(&self, location: Location) -> Option<BlockId> {
        self.location_to_block.get(&location).copied()
    }

    pub fn phi_blocks(
        &self,
        phi_values: &BTreeMap<MergeIdentity, SsaValueId>,
    ) -> Result<BTreeMap<SsaValueId, BlockId>, MokaIRBuildError> {
        phi_values
            .iter()
            .map(|(identity, &value)| {
                self.block_at(identity.location)
                    .map(|block| (value, block))
                    .ok_or(MokaIRBuildError::MalformedControlFlow)
            })
            .collect()
    }

    pub fn has_entry_preheader(&self) -> bool {
        self.entry != self.bytecode_entry
    }

    pub const fn bytecode_entry(&self) -> BlockId {
        self.bytecode_entry
    }
}

fn predecessor_locations(
    locations: &BTreeMap<Location, AnalyzedLocation>,
) -> BTreeMap<Location, BTreeSet<Location>> {
    let mut predecessors: BTreeMap<Location, BTreeSet<Location>> = BTreeMap::new();
    for (&source, facts) in locations {
        for outgoing in &facts.outgoing {
            predecessors
                .entry(outgoing.target)
                .or_default()
                .insert(source);
        }
    }
    predecessors
}

fn discover_leaders(
    analyzed_cfg: &AnalyzedJvmCfg,
    reachable: &[Location],
    predecessors: &BTreeMap<Location, BTreeSet<Location>>,
) -> Result<BTreeSet<Location>, MokaIRBuildError> {
    let mut leaders = BTreeSet::from([analyzed_cfg.entry_location]);
    leaders.extend(
        analyzed_cfg
            .phi_values
            .keys()
            .map(|identity| identity.location),
    );
    leaders.extend(
        reachable
            .iter()
            .copied()
            .filter(|location| !matches!(location, Location::Bytecode { .. })),
    );
    leaders.extend(
        predecessors
            .iter()
            .filter(|(_, sources)| sources.len() > 1)
            .map(|(target, _)| *target),
    );

    for facts in analyzed_cfg.locations.values() {
        if facts.instruction.is_explicit_transfer()
            || facts.outgoing.iter().any(|outgoing| {
                matches!(
                    outgoing.transfer,
                    ControlTransfer::Normal
                        | ControlTransfer::Exception(_)
                        | ControlTransfer::Unwind
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
        let plain_fallthrough = !facts.instruction.is_explicit_transfer()
            && facts.outgoing.len() == 1
            && facts.outgoing[0].target == *next
            && matches!(facts.outgoing[0].transfer, ControlTransfer::Unconditional);
        if !plain_fallthrough {
            leaders.insert(*next);
        }
    }
    leaders.retain(|location| analyzed_cfg.locations.contains_key(location));
    Ok(leaders)
}

fn allocate_block_ids(
    leaders: &BTreeSet<Location>,
    entry_location: Location,
    needs_entry_preheader: bool,
) -> Result<(BlockId, BlockId, BTreeMap<Location, BlockId>), MokaIRBuildError> {
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
        BlockId::new(0)
    } else {
        bytecode_entry
    };
    Ok((entry, bytecode_entry, block_ids))
}

struct GroupedLocations {
    block_by_location: BTreeMap<Location, BlockId>,
    locations_by_block: BTreeMap<BlockId, Vec<Location>>,
}

fn group_locations(
    reachable: &[Location],
    block_ids: &BTreeMap<Location, BlockId>,
) -> Result<GroupedLocations, MokaIRBuildError> {
    let mut block_by_location = BTreeMap::new();
    let mut locations_by_block: BTreeMap<BlockId, Vec<Location>> = BTreeMap::new();
    let mut current_block = None;
    for &location in reachable {
        if let Some(id) = block_ids.get(&location) {
            current_block = Some(*id);
        }
        let id = current_block.ok_or(MokaIRBuildError::MalformedControlFlow)?;
        block_by_location.insert(location, id);
        locations_by_block.entry(id).or_default().push(location);
    }
    Ok(GroupedLocations {
        block_by_location,
        locations_by_block,
    })
}
