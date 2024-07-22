//! Implementations for the traits in the `petgraph` crate.

use std::collections::BTreeSet;

use petgraph::{
    visit::{
        Data, GraphBase, GraphProp, IntoEdgeReferences, IntoNeighbors, IntoNeighborsDirected,
        IntoNodeIdentifiers, IntoNodeReferences, NodeIndexable, VisitMap, Visitable,
    },
    Directed, Direction,
};

use crate::{ir::ControlFlowGraph, jvm::code::ProgramCounter};

impl<N, E> Data for ControlFlowGraph<N, E> {
    type NodeWeight = N;
    type EdgeWeight = E;
}

impl<'a, N, E> IntoNodeReferences for &'a ControlFlowGraph<N, E> {
    type NodeRef = (ProgramCounter, &'a Self::NodeWeight);

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type NodeReferences = <Vec<Self::NodeRef> as IntoIterator>::IntoIter;

    fn node_references(self) -> Self::NodeReferences {
        self.nodes().collect::<Vec<_>>().into_iter()
    }
}

impl<'a, N, E> IntoEdgeReferences for &'a ControlFlowGraph<N, E> {
    type EdgeRef = (Self::NodeId, Self::NodeId, &'a Self::EdgeWeight);

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type EdgeReferences = <Vec<Self::EdgeRef> as IntoIterator>::IntoIter;

    fn edge_references(self) -> Self::EdgeReferences {
        self.edges().collect::<Vec<_>>().into_iter()
    }
}

impl<N, E> GraphBase for ControlFlowGraph<N, E> {
    type NodeId = ProgramCounter;
    type EdgeId = (ProgramCounter, ProgramCounter);
}

/// A visit map for the control flow graph.
pub type Visited = BTreeSet<ProgramCounter>;

impl VisitMap<ProgramCounter> for Visited {
    fn visit(&mut self, a: ProgramCounter) -> bool {
        self.insert(a)
    }

    fn is_visited(&self, a: &ProgramCounter) -> bool {
        self.contains(a)
    }
}

impl<N, E> Visitable for ControlFlowGraph<N, E> {
    type Map = Visited;

    fn visit_map(&self) -> Self::Map {
        BTreeSet::new()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}

impl<'a, N, E> IntoNodeIdentifiers for &'a ControlFlowGraph<N, E> {
    type NodeIdentifiers = <BTreeSet<Self::NodeId> as IntoIterator>::IntoIter;

    fn node_identifiers(self) -> Self::NodeIdentifiers {
        self.inner
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}

impl<'a, N, E> IntoNeighbors for &'a ControlFlowGraph<N, E> {
    type Neighbors = <BTreeSet<Self::NodeId> as IntoIterator>::IntoIter;

    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        self.neighbors_directed(a, Direction::Outgoing)
    }
}

impl<'a, N, E> IntoNeighborsDirected for &'a ControlFlowGraph<N, E> {
    type NeighborsDirected = <BTreeSet<Self::NodeId> as IntoIterator>::IntoIter;

    fn neighbors_directed(self, n: Self::NodeId, d: Direction) -> Self::NeighborsDirected {
        if d == Direction::Outgoing {
            self.inner
                .get(&n)
                .map(|(_, edges)| edges.keys().copied())
                .unwrap_or_default()
                .collect::<BTreeSet<_>>()
                .into_iter()
        } else {
            self.inner
                .iter()
                .flat_map(|(src, (_, outgoing_edges))| {
                    outgoing_edges.iter().map(|(dst, data)| (*src, *dst, data))
                })
                .filter(|(_, dst, _)| *dst == n)
                .map(|(src, _, _)| src)
                .collect::<BTreeSet<_>>()
                .into_iter()
        }
    }
}

impl<N, E> NodeIndexable for ControlFlowGraph<N, E> {
    fn node_bound(&self) -> usize {
        self.inner
            .last_key_value()
            .map(|(n, _)| u16::from(*n).into())
            .unwrap_or_default()
    }

    fn to_index(&self, ix: Self::NodeId) -> usize {
        usize::from(u16::from(ix))
    }

    fn from_index(&self, ix: usize) -> Self::NodeId {
        u16::try_from(ix).expect("Index is out of u16").into()
    }
}

impl<N, E> GraphProp for ControlFlowGraph<N, E> {
    type EdgeType = Directed;
}
