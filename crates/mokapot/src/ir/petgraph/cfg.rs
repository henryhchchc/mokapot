//! Petgraph support for Moka IR control-flow views.

use std::collections::HashSet;

use petgraph::{
    Directed, Direction,
    visit::{
        Data, EdgeRef, GraphBase, GraphProp, IntoEdgeReferences, IntoNeighbors,
        IntoNeighborsDirected, IntoNodeIdentifiers, IntoNodeReferences, NodeIndexable, Visitable,
    },
};

use crate::ir::{
    BasicBlock, BlockId, EdgeId,
    control_flow::{ControlFlowGraph, ControlTransfer, Edge},
};

impl Data for ControlFlowGraph<'_> {
    type NodeWeight = BasicBlock;
    type EdgeWeight = ControlTransfer;
}

impl<'method> IntoNodeReferences for &ControlFlowGraph<'method> {
    type NodeRef = (BlockId, &'method BasicBlock);
    type NodeReferences = <Vec<Self::NodeRef> as IntoIterator>::IntoIter;

    fn node_references(self) -> Self::NodeReferences {
        (*self).nodes().collect::<Vec<_>>().into_iter()
    }
}

impl<'method> IntoEdgeReferences for &ControlFlowGraph<'method> {
    type EdgeRef = Edge<'method>;
    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type EdgeReferences = <Vec<Self::EdgeRef> as IntoIterator>::IntoIter;

    fn edge_references(self) -> Self::EdgeReferences {
        (*self).edges().collect::<Vec<_>>().into_iter()
    }
}

impl EdgeRef for Edge<'_> {
    type NodeId = BlockId;
    type EdgeId = EdgeId;
    type Weight = ControlTransfer;

    fn source(&self) -> Self::NodeId {
        (*self).source()
    }

    fn target(&self) -> Self::NodeId {
        (*self).target()
    }

    fn weight(&self) -> &Self::Weight {
        (*self).transfer()
    }

    fn id(&self) -> Self::EdgeId {
        (*self).id()
    }
}

impl GraphBase for ControlFlowGraph<'_> {
    type NodeId = BlockId;
    type EdgeId = EdgeId;
}

impl Visitable for ControlFlowGraph<'_> {
    type Map = HashSet<BlockId>;

    fn visit_map(&self) -> Self::Map {
        HashSet::new()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}

impl IntoNodeIdentifiers for &ControlFlowGraph<'_> {
    type NodeIdentifiers = <Vec<BlockId> as IntoIterator>::IntoIter;

    fn node_identifiers(self) -> Self::NodeIdentifiers {
        self.blocks
            .iter()
            .map(BasicBlock::id)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl IntoNeighbors for &ControlFlowGraph<'_> {
    type Neighbors = <Vec<BlockId> as IntoIterator>::IntoIter;

    fn neighbors(self, block: BlockId) -> Self::Neighbors {
        self.neighbors_directed(block, Direction::Outgoing)
    }
}

impl IntoNeighborsDirected for &ControlFlowGraph<'_> {
    type NeighborsDirected = <Vec<BlockId> as IntoIterator>::IntoIter;

    fn neighbors_directed(self, block: BlockId, direction: Direction) -> Self::NeighborsDirected {
        if direction == Direction::Outgoing {
            self.blocks
                .get(usize::try_from(block.index()).unwrap_or(usize::MAX))
                .into_iter()
                .flat_map(|block| block.terminator().successors())
                .map(super::super::Successor::target)
                .collect::<Vec<_>>()
                .into_iter()
        } else {
            self.blocks
                .iter()
                .flat_map(|candidate| {
                    candidate
                        .terminator()
                        .successors()
                        .iter()
                        .filter(move |successor| successor.target() == block)
                        .map(move |_| candidate.id())
                })
                .collect::<Vec<_>>()
                .into_iter()
        }
    }
}

impl NodeIndexable for ControlFlowGraph<'_> {
    fn node_bound(&self) -> usize {
        self.blocks.len()
    }

    fn to_index(&self, block: Self::NodeId) -> usize {
        usize::try_from(block.index()).expect("block identity must fit usize")
    }

    fn from_index(&self, index: usize) -> Self::NodeId {
        BlockId::new(u32::try_from(index).expect("block index must fit u32"))
    }
}

impl GraphProp for ControlFlowGraph<'_> {
    type EdgeType = Directed;
}

#[cfg(test)]
mod tests;
