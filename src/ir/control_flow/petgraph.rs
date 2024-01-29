//! Implementations for the traits in the `petgraph` crate.

use std::collections::{BTreeSet, HashSet};

use itertools::Itertools;
use petgraph::{
    visit::{
        Data, GraphBase, GraphProp, IntoEdgeReferences, IntoNeighbors, IntoNeighborsDirected,
        IntoNodeIdentifiers, IntoNodeReferences, NodeIndexable, NodeRef, Reversed, VisitMap,
        Visitable,
    },
    Directed, Direction,
};

use crate::jvm::code::ProgramCounter;

use super::{ControlFlowEdgeKind, ControlFlowGraph};

impl Data for ControlFlowGraph {
    type NodeWeight = ProgramCounter;
    type EdgeWeight = ControlFlowEdgeKind;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CFGNode(ProgramCounter, ProgramCounter);

impl NodeRef for CFGNode {
    type NodeId = ProgramCounter;

    type Weight = ProgramCounter;

    fn id(&self) -> Self::NodeId {
        self.0
    }

    fn weight(&self) -> &Self::Weight {
        &self.1
    }
}

impl<'a> IntoNodeReferences for &'a ControlFlowGraph {
    type NodeRef = CFGNode;

    type NodeReferences = <BTreeSet<CFGNode> as IntoIterator>::IntoIter;

    fn node_references(self) -> Self::NodeReferences {
        self.node_identifiers()
            .map(|it| CFGNode(it, it))
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}

impl<'a> IntoEdgeReferences for &'a ControlFlowGraph {
    type EdgeRef = (Self::NodeId, Self::NodeId, &'a Self::EdgeWeight);

    type EdgeReferences = <HashSet<Self::EdgeRef> as IntoIterator>::IntoIter;

    fn edge_references(self) -> Self::EdgeReferences {
        self.inner
            .iter()
            .map(|((src, dst), kind)| (*src, *dst, kind))
            .collect::<HashSet<_>>()
            .into_iter()
    }
}

impl GraphBase for ControlFlowGraph {
    type NodeId = ProgramCounter;
    type EdgeId = (ProgramCounter, ProgramCounter);
}

pub type Visited = BTreeSet<ProgramCounter>;

impl VisitMap<ProgramCounter> for Visited {
    fn visit(&mut self, a: ProgramCounter) -> bool {
        self.insert(a)
    }

    fn is_visited(&self, a: &ProgramCounter) -> bool {
        self.contains(a)
    }
}

impl Visitable for ControlFlowGraph {
    type Map = Visited;

    fn visit_map(&self) -> Self::Map {
        BTreeSet::new()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}

impl<'a> IntoNodeIdentifiers for &'a ControlFlowGraph {
    type NodeIdentifiers = <BTreeSet<Self::NodeId> as IntoIterator>::IntoIter;

    fn node_identifiers(self) -> Self::NodeIdentifiers {
        self.inner
            .keys()
            .copied()
            .flat_map(|(src, dst)| [src, dst])
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}

impl<'a> IntoNeighbors for &'a ControlFlowGraph {
    type Neighbors = <BTreeSet<Self::NodeId> as IntoIterator>::IntoIter;

    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        self.inner
            .keys()
            .copied()
            .filter(|(src, _)| *src == a)
            .map(|(_, dst)| dst)
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}

impl<'a> IntoNeighborsDirected for &'a ControlFlowGraph {
    type NeighborsDirected = <BTreeSet<Self::NodeId> as IntoIterator>::IntoIter;

    fn neighbors_directed(self, n: Self::NodeId, d: Direction) -> Self::NeighborsDirected {
        match d {
            Direction::Outgoing => self.neighbors(n),
            Direction::Incoming => Reversed(self).neighbors(n),
        }
    }
}

impl NodeIndexable for ControlFlowGraph {
    fn node_bound(&self) -> usize {
        self.inner
            .keys()
            .copied()
            .flat_map(|(src, dst)| [src, dst])
            .dedup()
            .count()
    }

    fn to_index(&self, ix: Self::NodeId) -> usize {
        usize::from(u16::from(ix))
    }

    fn from_index(&self, ix: usize) -> Self::NodeId {
        u16::try_from(ix).expect("Index is out of u16").into()
    }
}

impl GraphProp for ControlFlowGraph {
    type EdgeType = Directed;
}
