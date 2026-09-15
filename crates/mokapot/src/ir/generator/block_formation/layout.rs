//! Discovers the structural layout of semantic JVM blocks.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    BlockId,
    generator::{
        bytecode_analysis::{self, FrameMergeSite, Node, NodeAddress, jvm::Frame},
        error::Error,
        identity::SsaValueId,
    },
};

/// The partition of JVM instruction nodes into semantic blocks.
///
/// Block ids are dense and ascend with the address of the first node, so a
/// block's id is also its index in the materialized block vector.
pub(super) struct BlockLayout {
    /// The block that every path enters the method through.
    pub(super) entry: BlockId,
    /// The blocks of the method, in ascending id order.
    pub(super) blocks: Vec<LayoutBlock>,
    /// The block that owns each address.
    pub(super) block_by_addr: BTreeMap<NodeAddress, BlockId>,
    /// The block that computes each frame merge site.
    pub(super) phi_blocks: BTreeMap<FrameMergeSite, BlockId>,
}

/// The contents of one block of a [`BlockLayout`].
pub(super) enum LayoutBlock {
    /// The synthetic entry preheader, which enters the method frame and hands
    /// it to the bytecode entry block.
    EntryPreheader {
        frame: Frame<bytecode_analysis::Value>,
        target: BlockId,
    },
    /// The nodes of a block, in execution order.
    Nodes(Vec<(NodeAddress, Node)>),
}

impl BlockLayout {
    /// Discovers the layout of a complete instruction graph.
    ///
    /// `nodes` is partitioned, so materialization cannot find a leftover node;
    /// a merge site whose address has no node is malformed, because no block
    /// could compute it.
    pub(super) fn discover(
        nodes: BTreeMap<NodeAddress, Node>,
        entry_addr: NodeAddress,
        initial_frame: Frame<bytecode_analysis::Value>,
        phi_values: &BTreeMap<FrameMergeSite, SsaValueId>,
    ) -> Result<Self, Error> {
        let facts = Facts::derive(&nodes, entry_addr, phi_values)?;
        let leaders = facts.leaders()?;
        let needs_entry_preheader = facts.needs_entry_preheader();
        let block_ids = allocate_block_ids(&leaders, needs_entry_preheader)?;

        let mut block_by_addr = BTreeMap::new();
        let mut walked = Vec::with_capacity(nodes.len());
        for &leader in &leaders {
            let block = block_ids[&leader];
            for addr in facts.chain(leader, &leaders) {
                block_by_addr.insert(addr, block);
                walked.push(addr);
            }
        }
        // Elision is injective, so chains are disjoint and every non-leader
        // belongs to exactly one: walking the leaders in ascending order visits
        // the whole method in address order.
        debug_assert_eq!(
            walked, facts.addresses,
            "chains must partition the method's nodes in address order"
        );

        let bytecode_entry = *block_ids
            .get(&entry_addr)
            .expect("the entry address always begins a block");
        let entry = if needs_entry_preheader {
            BlockId::new(0)
        } else {
            bytecode_entry
        };
        // Every site is a leader, so `block_ids` already holds its address.
        let phi_blocks = phi_values
            .keys()
            .map(|&site| (site, block_ids[&site.addr]))
            .collect();

        let block_count = leaders.len() + usize::from(needs_entry_preheader);
        let mut blocks = partition(nodes, &block_by_addr, block_count)?;
        if needs_entry_preheader {
            // The preheader owns no nodes, so its slot is still empty.
            blocks[0] = LayoutBlock::EntryPreheader {
                frame: initial_frame,
                target: bytecode_entry,
            };
        }

        Ok(Self {
            entry,
            blocks,
            block_by_addr,
            phi_blocks,
        })
    }
}

/// The control-flow facts block discovery derives, once, from an instruction
/// graph.
///
/// Which locations begin a block, which end one, and whether the entry needs a
/// preheader are all read from this one traversal, not re-derived.
struct Facts<'graph> {
    /// Every node of the graph.
    nodes: &'graph BTreeMap<NodeAddress, Node>,
    /// Every node address, in address order: bytecode locations by program
    /// counter then expansion context, then the handler and unwind locations.
    ///
    /// A fallthrough reaches the location after it in this order, and an edge
    /// into a block interior is impossible, because it would have made its
    /// target a leader.
    addresses: Vec<NodeAddress>,
    /// The sources of every address reached by an edge.
    predecessors: BTreeMap<NodeAddress, BTreeSet<NodeAddress>>,
    /// The address that the method is entered at.
    entry_addr: NodeAddress,
    /// The frame merge sites that the frames of the graph carry.
    phi_values: &'graph BTreeMap<FrameMergeSite, SsaValueId>,
}

impl<'graph> Facts<'graph> {
    /// Derives the facts of every node of `nodes`.
    fn derive(
        nodes: &'graph BTreeMap<NodeAddress, Node>,
        entry_addr: NodeAddress,
        phi_values: &'graph BTreeMap<FrameMergeSite, SsaValueId>,
    ) -> Result<Self, Error> {
        if nodes.is_empty() {
            return Err(Error::internal("the instruction graph is empty"));
        }
        let addresses = nodes.keys().copied().collect();
        let mut predecessors: BTreeMap<NodeAddress, BTreeSet<NodeAddress>> = BTreeMap::new();
        for (&source, node) in nodes {
            for edge in &node.outgoing_edges {
                predecessors.entry(edge.target).or_default().insert(source);
            }
        }
        // The walks follow edge targets, which need not be leaders, so a target
        // the graph does not hold is malformed and would panic later.
        if predecessors.keys().any(|addr| !nodes.contains_key(addr)) {
            return Err(Error::internal(
                "an instruction edge targets a missing node",
            ));
        }
        Ok(Self {
            nodes,
            addresses,
            predecessors,
            entry_addr,
            phi_values,
        })
    }

    /// The node at `addr`, which the checks in `derive` and `leaders` prove to
    /// exist.
    fn node(&self, addr: NodeAddress) -> &'graph Node {
        let nodes = self.nodes;
        &nodes[&addr]
    }

    /// The address that `addr` is elided into, if it is elided at all.
    fn elision_target(&self, addr: NodeAddress) -> Option<NodeAddress> {
        let node = self.node(addr);
        node.outgoing_edges
            .iter()
            .map(|edge| edge.target)
            .find(|&target| node.elides_into(target))
    }

    /// The addresses of the block that begins at `leader`.
    ///
    /// A chain follows the elision of each of its nodes and stops before it
    /// consumes a leader, which begins a block of its own.
    fn chain(&self, leader: NodeAddress, leaders: &BTreeSet<NodeAddress>) -> Vec<NodeAddress> {
        let mut chain = vec![leader];
        let mut current = leader;
        while let Some(next) = self.elision_target(current) {
            if leaders.contains(&next) {
                break;
            }
            chain.push(next);
            current = next;
        }
        chain
    }

    /// The addresses that begin a block.
    ///
    /// Every leader must be a node, because a chain is walked from each one.
    /// Edge targets are checked in `derive`, so this covers the remainder: the
    /// entry address and the merge sites.
    fn leaders(&self) -> Result<BTreeSet<NodeAddress>, Error> {
        let mut leaders = BTreeSet::from([self.entry_addr]);
        leaders.extend(self.phi_values.keys().map(|site| site.addr));
        // A handler entry introduces the caught exception before its guarded
        // bytecode runs, and the unwind exit ends the method. Neither may be
        // elided into a predecessor, so both begin a block.
        leaders.extend(
            self.addresses
                .iter()
                .copied()
                .filter(|&addr| addr.is_handler() || matches!(addr, NodeAddress::Unwind)),
        );
        leaders.extend(
            self.predecessors
                .iter()
                .filter(|(_, sources)| sources.len() > 1)
                .map(|(&target, _)| target),
        );

        let mut previous = None;
        for &addr in &self.addresses {
            let node = self.node(addr);
            // A location that ends its block hands every arm to a fresh block.
            // Synchronously fallible operations qualify through their
            // exceptional arms, which they always have.
            if node.instruction.is_explicit_transfer() || node.has_exceptional_exit() {
                leaders.extend(node.outgoing_edges.iter().map(|edge| edge.target));
            }
            // A location that is elided into the one after it continues its
            // block; the one after it begins a block wherever it is not.
            if let Some(previous) = previous
                && !self.node(previous).elides_into(addr)
            {
                leaders.insert(addr);
            }
            previous = Some(addr);
        }
        if leaders.iter().any(|addr| !self.nodes.contains_key(addr)) {
            return Err(Error::internal("a block leader has no instruction node"));
        }
        Ok(leaders)
    }

    /// Whether the method entry is reached by an edge, and therefore needs a
    /// synthetic preheader to enter the method frame.
    fn needs_entry_preheader(&self) -> bool {
        self.predecessors.contains_key(&self.entry_addr)
    }
}

/// Allocates a dense block id to every leader.
///
/// Ids follow the ascending leader order, offset by one for the synthetic entry
/// preheader, so that the numbering of the blocks is deterministic and their
/// ids index them directly.
fn allocate_block_ids(
    leaders: &BTreeSet<NodeAddress>,
    needs_entry_preheader: bool,
) -> Result<BTreeMap<NodeAddress, BlockId>, Error> {
    let block_offset = u32::from(needs_entry_preheader);
    leaders
        .iter()
        .enumerate()
        .map(|(index, addr)| {
            u32::try_from(index)
                .ok()
                .and_then(|index| index.checked_add(block_offset))
                .map(|index| (*addr, BlockId::new(index)))
                .ok_or_else(|| Error::internal("the block identity space is exhausted"))
        })
        .collect()
}

/// Partitions `nodes` into their blocks.
///
/// A node outside `block_of` and a block id outside `block_count` are both
/// malformed. Iterating in address order keeps each block's nodes in execution
/// order.
fn partition(
    nodes: BTreeMap<NodeAddress, Node>,
    block_of: &BTreeMap<NodeAddress, BlockId>,
    block_count: usize,
) -> Result<Vec<LayoutBlock>, Error> {
    let mut blocks = (0..block_count).map(|_| Vec::new()).collect::<Vec<_>>();
    for (addr, node) in nodes {
        let block = block_of
            .get(&addr)
            .ok_or_else(|| Error::internal("an instruction node has no owning block"))?;
        let index = usize::try_from(block.index())
            .map_err(|_| Error::internal("an allocated block identity cannot be addressed"))?;
        blocks
            .get_mut(index)
            .ok_or_else(|| Error::internal("an allocated block identity is out of range"))?
            .push((addr, node));
    }
    Ok(blocks.into_iter().map(LayoutBlock::Nodes).collect())
}
