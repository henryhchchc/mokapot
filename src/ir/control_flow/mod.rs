//! Control flow analysis

pub mod path_condition;

use crate::{
    analysis::fixed_point::Analyzer,
    jvm::{code::ProgramCounter, references::ClassRef},
};
use std::collections::{BTreeMap, BTreeSet};

use self::path_condition::{PathCondition, Predicate, Value};

use super::ControlFlowGraph;

/// The kind of a control transfer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlTransfer {
    /// An unconditional control transfer.
    Unconditional,
    /// A conditional contol transfer.
    Conditional(PathCondition<Predicate<Value>>),
    /// A control transfer to the exception handler.
    Exception(BTreeSet<ClassRef>),
    /// A control transfer caused by subroutine return.
    SubroutineReturn,
}

impl<N, E> ControlFlowGraph<N, E> {
    /// Returns the entry point of the control flow graph.
    #[must_use]
    pub const fn entry_point(&self) -> ProgramCounter {
        ProgramCounter::ZERO
    }

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
    pub fn nodes(&self) -> impl Iterator<Item = (ProgramCounter, &N)> {
        self.inner.iter().map(|(n, (d, _))| (*n, d))
    }

    /// Returns an iterator over the edges
    pub fn edges(&self) -> impl Iterator<Item = (ProgramCounter, ProgramCounter, &E)> {
        self.inner.iter().flat_map(|(src, (_, outgoing_edges))| {
            outgoing_edges.iter().map(|(dst, data)| (*src, *dst, data))
        })
    }

    /// Returns an iterator over the exits of the control flow graph.
    pub fn exits(&self) -> impl Iterator<Item = ProgramCounter> + '_ {
        self.inner
            .iter()
            .filter(|(_, (_, outgoing_edges))| outgoing_edges.is_empty())
            .map(|(n, _)| *n)
    }

    /// Returns an iterator over the edges starting at the given node.
    #[must_use]
    pub fn edges_from(
        &self,
        src: ProgramCounter,
    ) -> Option<impl Iterator<Item = (ProgramCounter, ProgramCounter, &E)>> {
        self.inner.get(&src).map(|(_, outgoing_edges)| {
            outgoing_edges
                .iter()
                .map(move |(dst, data)| (src, *dst, data))
        })
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
        let mut inner: BTreeMap<_, (_, BTreeMap<_, _>)> = BTreeMap::new();
        inner.entry(ProgramCounter::ZERO).or_default();
        edges.into_iter().for_each(|(src, dst, data)| {
            let ((), edge_map) = inner.entry(src).or_default();
            assert!(edge_map.insert(dst, data).is_none(), "Duplicate edge");
            inner.entry(dst).or_default();
        });
        Self { inner }
    }
}

impl ControlFlowGraph<(), ControlTransfer> {
    /// Analyzes the control flow graph to determine the path conditions at each program counter.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn path_conditions(&self) -> BTreeMap<ProgramCounter, PathCondition<Predicate<Value>>> {
        let mut analyzer = path_condition::Analyzer::new(self);
        analyzer.analyze().expect("Never panics")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn entry_point() {
        let cfg = ControlFlowGraph::<(), ()>::from_edges(vec![]);
        assert_eq!(cfg.entry_point(), ProgramCounter::ZERO);
    }

    fn build_cfg() -> ControlFlowGraph<(), ()> {
        let edges = [
            (0.into(), 1.into(), ()),
            (1.into(), 2.into(), ()),
            (2.into(), 3.into(), ()),
            (3.into(), 4.into(), ()),
        ];
        ControlFlowGraph::from_edges(edges)
    }

    #[test]
    #[should_panic(expected = "Duplicate edge")]
    fn from_edges_duplicate() {
        let edges = [
            (0.into(), 1.into(), ()),
            (1.into(), 2.into(), ()),
            (2.into(), 3.into(), ()),
            (3.into(), 4.into(), ()),
            (0.into(), 1.into(), ()),
        ];
        ControlFlowGraph::from_edges(edges);
    }

    #[test]
    fn iter_nodes() {
        let cfg = build_cfg();
        let nodes = cfg.nodes().collect::<BTreeSet<_>>();
        assert_eq!(nodes.len(), 5);
        for i in 0..=4 {
            assert!(nodes.contains(&(i.into(), &())));
        }
    }

    #[test]
    fn iter_edges() {
        let cfg = build_cfg();
        let edges = cfg.edges().collect::<BTreeSet<_>>();
        assert_eq!(edges.len(), 4);
        for i in 0..=3 {
            assert!(edges.contains(&(i.into(), (i + 1).into(), &())));
        }
    }

    #[test]
    fn iter_exits() {
        let cfg = build_cfg();
        let exits = cfg.exits().collect::<BTreeSet<_>>();
        assert_eq!(exits.len(), 1);
        assert!(exits.contains(&4.into()));
    }
}
