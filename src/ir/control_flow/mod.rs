//! Control flow analysis

use crate::jvm::{class::ClassReference, code::ProgramCounter};
use std::collections::BTreeMap;

#[cfg(feature = "petgraph")]
pub mod petgraph;

/// A control flow edge in a control flow graph
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ControlFlowEdge<E> {
    pub(super) src: ProgramCounter,
    pub(super) dst: ProgramCounter,
    pub(super) data: E,
}

impl ControlFlowEdge<ControlTransfer> {
    pub(super) fn unconditional(source: ProgramCounter, destination: ProgramCounter) -> Self {
        Self {
            src: source,
            dst: destination,
            data: ControlTransfer::Unconditional,
        }
    }

    pub(super) fn conditional(source: ProgramCounter, destination: ProgramCounter) -> Self {
        Self {
            src: source,
            dst: destination,
            data: ControlTransfer::Conditional,
        }
    }

    pub(super) fn exception(
        source: ProgramCounter,
        destination: ProgramCounter,
        exception: ClassReference,
    ) -> Self {
        Self {
            src: source,
            dst: destination,
            data: ControlTransfer::Exception(exception),
        }
    }

    pub(super) fn subroutine_return(source: ProgramCounter, destination: ProgramCounter) -> Self {
        Self {
            src: source,
            dst: destination,
            data: ControlTransfer::SubroutineReturn,
        }
    }
}

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
    node_data: BTreeMap<ProgramCounter, N>,
    edge_data: BTreeMap<(ProgramCounter, ProgramCounter), E>,
}

impl<N, E> ControlFlowGraph<N, E> {
    /// Transforms the node and edge data to construt a new control flow graph.
    #[must_use]
    pub fn map<N1, E1, NMap, EMap>(self, nf: NMap, ef: EMap) -> ControlFlowGraph<N1, E1>
    where
        NMap: Fn(ProgramCounter, N) -> N1,
        EMap: Fn((ProgramCounter, ProgramCounter), E) -> E1,
    {
        let node_data = self
            .node_data
            .into_iter()
            .map(|(pc, data)| (pc, nf(pc, data)))
            .collect();
        let edge_data = self
            .edge_data
            .into_iter()
            .map(|(edge, data)| (edge, ef(edge, data)))
            .collect();
        ControlFlowGraph {
            node_data,
            edge_data,
        }
    }
}

impl ControlFlowGraph<(), ControlTransfer> {
    pub(super) fn from_edges(
        edges: impl IntoIterator<Item = ControlFlowEdge<ControlTransfer>>,
    ) -> Self {
        let mut edge_data = BTreeMap::new();
        edges.into_iter().for_each(
            |ControlFlowEdge {
                 src,
                 dst,
                 data: kind,
             }| {
                assert!(
                    edge_data.insert((src, dst), kind).is_none(),
                    "Duplitcate edge"
                );
            },
        );
        let node_data = edge_data
            .keys()
            .flat_map(|(src, dst)| [*src, *dst])
            .map(|it| (it, ()))
            .collect();
        Self {
            node_data,
            edge_data,
        }
    }
}
