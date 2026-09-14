//! Discovers the structural layout of semantic JVM blocks.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    BlockId,
    control_flow::ControlTransfer,
    generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            NodeAddress,
            symbolic_execution::{self, FrameMergeSite},
        },
    },
};

/// The partition of symbolic JVM nodes into semantic blocks.
pub(super) struct BlockLayout {
    entry: BlockId,
    bytecode_entry: BlockId,
    addr_to_block: BTreeMap<NodeAddress, BlockId>,
    addrs_by_block: BTreeMap<BlockId, Vec<NodeAddress>>,
}

impl BlockLayout {
    pub fn discover(symbolic_cfg: &symbolic_execution::Cfg) -> Result<Self, MokaIRBuildError> {
        let reachable = symbolic_cfg.nodes.keys().copied().collect::<Vec<_>>();
        if reachable.is_empty() {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }

        let predecessors = predecessor_addrs(&symbolic_cfg.nodes);
        let leaders = discover_leaders(symbolic_cfg, &reachable, &predecessors)?;
        let needs_entry_preheader = predecessors
            .get(&symbolic_cfg.entry_addr)
            .is_some_and(|sources| !sources.is_empty());
        let (entry, bytecode_entry, block_ids) =
            allocate_block_ids(&leaders, symbolic_cfg.entry_addr, needs_entry_preheader)?;
        let grouped = group_addrs(&reachable, &block_ids)?;

        Ok(Self {
            entry,
            bytecode_entry,
            addr_to_block: grouped.block_by_addr,
            addrs_by_block: grouped.addrs_by_block,
        })
    }

    pub const fn entry(&self) -> BlockId {
        self.entry
    }

    pub const fn addrs(&self) -> &BTreeMap<BlockId, Vec<NodeAddress>> {
        &self.addrs_by_block
    }

    pub fn block_at(&self, addr: NodeAddress) -> Option<BlockId> {
        self.addr_to_block.get(&addr).copied()
    }

    pub fn phi_blocks(
        &self,
        phi_values: &BTreeMap<FrameMergeSite, SsaValueId>,
    ) -> Result<BTreeMap<SsaValueId, BlockId>, MokaIRBuildError> {
        phi_values
            .iter()
            .map(|(identity, &value)| {
                self.block_at(identity.addr)
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

fn predecessor_addrs(
    addrs: &BTreeMap<NodeAddress, symbolic_execution::Node>,
) -> BTreeMap<NodeAddress, BTreeSet<NodeAddress>> {
    let mut predecessors: BTreeMap<NodeAddress, BTreeSet<NodeAddress>> = BTreeMap::new();
    for (&source, facts) in addrs {
        for outgoing in &facts.outgoing_edges {
            predecessors
                .entry(outgoing.target)
                .or_default()
                .insert(source);
        }
    }
    predecessors
}

fn discover_leaders(
    symbolic_cfg: &symbolic_execution::Cfg,
    reachable: &[NodeAddress],
    predecessors: &BTreeMap<NodeAddress, BTreeSet<NodeAddress>>,
) -> Result<BTreeSet<NodeAddress>, MokaIRBuildError> {
    let mut leaders = BTreeSet::from([symbolic_cfg.entry_addr]);
    leaders.extend(symbolic_cfg.phi_values.keys().map(|identity| identity.addr));
    leaders.extend(
        reachable
            .iter()
            .copied()
            .filter(|addr| !matches!(addr, NodeAddress::Bytecode { .. })),
    );
    leaders.extend(
        predecessors
            .iter()
            .filter(|(_, sources)| sources.len() > 1)
            .map(|(target, _)| *target),
    );

    for facts in symbolic_cfg.nodes.values() {
        if facts.instruction.is_explicit_transfer()
            || facts.outgoing_edges.iter().any(|outgoing| {
                matches!(
                    outgoing.transfer,
                    ControlTransfer::Normal
                        | ControlTransfer::Exception(_)
                        | ControlTransfer::Unwind
                )
            })
        {
            leaders.extend(facts.outgoing_edges.iter().map(|outgoing| outgoing.target));
        }
    }
    for pair in reachable.windows(2) {
        let [current, next] = pair else {
            unreachable!()
        };
        let facts = symbolic_cfg
            .nodes
            .get(current)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let plain_fallthrough = !facts.instruction.is_explicit_transfer()
            && facts.outgoing_edges.len() == 1
            && facts.outgoing_edges[0].target == *next
            && matches!(
                facts.outgoing_edges[0].transfer,
                ControlTransfer::Unconditional
            );
        if !plain_fallthrough {
            leaders.insert(*next);
        }
    }
    leaders.retain(|addr| symbolic_cfg.nodes.contains_key(addr));
    Ok(leaders)
}

fn allocate_block_ids(
    leaders: &BTreeSet<NodeAddress>,
    entry_addr: NodeAddress,
    needs_entry_preheader: bool,
) -> Result<(BlockId, BlockId, BTreeMap<NodeAddress, BlockId>), MokaIRBuildError> {
    let block_offset = u32::from(needs_entry_preheader);
    let block_ids = leaders
        .iter()
        .enumerate()
        .map(|(index, addr)| {
            u32::try_from(index)
                .ok()
                .and_then(|index| index.checked_add(block_offset))
                .map(|index| (*addr, BlockId::new(index)))
                .ok_or(MokaIRBuildError::MalformedControlFlow)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let bytecode_entry = *block_ids
        .get(&entry_addr)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let entry = if needs_entry_preheader {
        BlockId::new(0)
    } else {
        bytecode_entry
    };
    Ok((entry, bytecode_entry, block_ids))
}

struct GroupedAddrs {
    block_by_addr: BTreeMap<NodeAddress, BlockId>,
    addrs_by_block: BTreeMap<BlockId, Vec<NodeAddress>>,
}

fn group_addrs(
    reachable: &[NodeAddress],
    block_ids: &BTreeMap<NodeAddress, BlockId>,
) -> Result<GroupedAddrs, MokaIRBuildError> {
    let mut block_by_addr = BTreeMap::new();
    let mut addrs_by_block: BTreeMap<BlockId, Vec<NodeAddress>> = BTreeMap::new();
    let mut current_block = None;
    for &addr in reachable {
        if let Some(id) = block_ids.get(&addr) {
            current_block = Some(*id);
        }
        let id = current_block.ok_or(MokaIRBuildError::MalformedControlFlow)?;
        block_by_addr.insert(addr, id);
        addrs_by_block.entry(id).or_default().push(addr);
    }
    Ok(GroupedAddrs {
        block_by_addr,
        addrs_by_block,
    })
}
