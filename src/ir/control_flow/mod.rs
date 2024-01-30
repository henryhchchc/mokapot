//! Control flow analysis

use crate::jvm::{class::ClassReference, code::ProgramCounter};
use std::collections::BTreeMap;

#[cfg(feature = "petgraph")]
pub mod petgraph;

/// The kind of a control transfer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControlTransfer {
    /// An unconditional control transfer.
    Unconditional,
    /// A conditional contol transfer.
    Conditional,
    /// A control transfer to the exception handler.
    Exception(ClassReference),
    /// A control transfer caused by subroutine return.
    SubroutineReturn,
}

/// A control flow graph.
///
/// It is generic over the data associated with each node and edge.
#[derive(Debug, Clone, Default)]
pub struct ControlFlowGraph<N, E> {
    inner: BTreeMap<ProgramCounter, (N, BTreeMap<ProgramCounter, E>)>,
}

impl<N, E> ControlFlowGraph<N, E> {
    /// Transforms the node and edge data to construt a new control flow graph.
    #[must_use]
    pub fn map<N1, E1, NMap, EMap>(self, nf: NMap, ef: EMap) -> ControlFlowGraph<N1, E1>
    where
        NMap: Fn(ProgramCounter, N) -> N1,
        EMap: Fn((ProgramCounter, ProgramCounter), E) -> E1,
    {
        let inner = self
            .inner
            .into_iter()
            .map(|(src, (node_data, edges))| {
                let data = nf(src, node_data);
                let edges = edges
                    .into_iter()
                    .map(|(dst, edge_data)| (dst, ef((src, dst), edge_data)))
                    .collect();
                (src, (data, edges))
            })
            .collect();

        ControlFlowGraph { inner }
    }

    /// Returns an iterator over the nodes
    pub fn iter_nodes(&self) -> impl Iterator<Item = (ProgramCounter, &N)> + '_ {
        self.inner.iter().map(|(n, (d, _))| (*n, d))
    }

    /// Returns an iterator over the edges
    pub fn iter_edges(&self) -> impl Iterator<Item = (ProgramCounter, ProgramCounter, &E)> + '_ {
        self.inner.iter().flat_map(|(src, (_, outgoing_edges))| {
            outgoing_edges.iter().map(|(dst, data)| (*src, *dst, data))
        })
    }

    /// Returns an iterator over the exits of the control flow graph.
    pub fn iter_exits(&self) -> impl Iterator<Item = ProgramCounter> + '_ {
        self.inner
            .iter()
            .filter(|(_, (_, outgoing_edges))| outgoing_edges.is_empty())
            .map(|(n, _)| *n)
    }
}

impl<E> ControlFlowGraph<(), E> {
    /// Constructs a new control flow graph from a set of edges.
    ///
    /// # Panics
    /// Panics if there are duplicate edges.
    pub fn from_edges(
        edges: impl IntoIterator<Item = (ProgramCounter, ProgramCounter, E)>,
    ) -> Self {
        let mut inner = BTreeMap::new();
        edges.into_iter().for_each(|(src, dst, data)| {
            assert!(
                inner
                    .entry(src)
                    .or_insert(((), BTreeMap::new()))
                    .1
                    .insert(dst, data)
                    .is_none(),
                "Duplicate edge"
            );
            inner.entry(dst).or_default();
        });
        Self { inner }
    }
}

#[test]
fn from_edges() {
    let edges = [
        (0.into(), 1.into(), ()),
        (1.into(), 2.into(), ()),
        (2.into(), 3.into(), ()),
        (3.into(), 4.into(), ()),
    ];
    let cfg = ControlFlowGraph::from_edges(edges);
    let nodes = cfg.iter_nodes().collect::<std::collections::BTreeSet<_>>();
    for i in 0..=4 {
        assert!(nodes.contains(&(i.into(), &())));
    }
    assert_eq!(cfg.iter_edges().count(), 4);
}
