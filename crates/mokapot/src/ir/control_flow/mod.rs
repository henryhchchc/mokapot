//! Control flow analysis

pub mod path_condition;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use self::path_condition::{PathCondition, Value};
use super::ControlFlowGraph;
use crate::{
    analysis::fixed_point::solve,
    ir::expression::Condition,
    jvm::{code::ProgramCounter, references::ClassRef},
};

/// The kind of a control transfer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControlTransfer {
    /// An unconditional control transfer.
    Unconditional,
    /// A conditional control transfer.
    Conditional(PathCondition<Condition<Value>>),
    /// A control transfer to the exception handler.
    Exception(BTreeSet<ClassRef>),
    /// A control transfer caused by subroutine return.
    SubroutineReturn,
}

/// An edge in the control flow graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Edge<D> {
    /// The source program counter.
    pub source: ProgramCounter,
    /// The target program counter.
    pub target: ProgramCounter,
    /// The data associated with the edge.
    pub data: D,
}

impl<D> Edge<D> {
    /// Creates a new edge with the given source, target, and data.
    pub const fn new(source: ProgramCounter, target: ProgramCounter, data: D) -> Self {
        Self {
            source,
            target,
            data,
        }
    }
}

impl<N, E> ControlFlowGraph<N, E> {
    /// Returns the entry point of the control flow graph.
    #[must_use]
    pub const fn entry_point(&self) -> ProgramCounter {
        ProgramCounter::ZERO
    }

    /// Transforms the node and edge data to construct a new control flow graph.
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
    pub fn edges(&self) -> impl Iterator<Item = Edge<&E>> {
        self.inner
            .iter()
            .flat_map(|(&source, (_, outgoing_edges))| {
                outgoing_edges.iter().map(move |(&target, data)| Edge {
                    source,
                    target,
                    data,
                })
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
    pub fn outgoing_edges(&self, from: ProgramCounter) -> Option<impl Iterator<Item = Edge<&E>>> {
        self.inner.get(&from).map(|(_, outgoing_edges)| {
            outgoing_edges.iter().map(move |(&target, data)| Edge {
                source: from,
                target,
                data,
            })
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

impl<N> ControlFlowGraph<N, ControlTransfer> {
    /// Analyzes the control flow graph to determine the path conditions at each program counter.
    /// # Performance
    /// The memory consumption is exponential in the number of unique predicates in this control flow graph.
    #[must_use]
    pub fn path_conditions(&self) -> HashMap<ProgramCounter, PathCondition<&Condition<Value>>> {
        let mut analyzer = path_condition::Analyzer::new(self);
        let Ok(path_conditions) = solve(&mut analyzer);
        path_conditions
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

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
        let edges = cfg.edges().collect::<HashSet<_>>();
        assert_eq!(edges.len(), 4);
        for i in 0..=3 {
            assert!(edges.contains(&Edge {
                source: i.into(),
                target: (i + 1).into(),
                data: &()
            }));
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
