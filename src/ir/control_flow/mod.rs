//! Control flow analysis

use crate::jvm::{class::ClassReference, code::ProgramCounter};
use std::collections::BTreeMap;

/// A control flow edge in the [`ControlFlowGraph`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ControlFlowEdge {
    /// The source of the edge.
    pub src: ProgramCounter,
    /// The destination of the edge.
    pub dst: ProgramCounter,
    /// The kind of the edge.
    pub kind: ControlFlowEdgeKind,
}

impl ControlFlowEdge {
    /// Creates a new unconditional control flow edge.
    #[must_use]
    pub fn unconditional(source: ProgramCounter, destination: ProgramCounter) -> Self {
        Self {
            src: source,
            dst: destination,
            kind: ControlFlowEdgeKind::Unconditional,
        }
    }

    /// Creates a new conditional control flow edge.
    #[must_use]
    pub fn conditional(source: ProgramCounter, destination: ProgramCounter) -> Self {
        Self {
            src: source,
            dst: destination,
            kind: ControlFlowEdgeKind::Conditional,
        }
    }

    /// Creates a new exception control flow edge.
    #[must_use]
    pub fn exception(
        source: ProgramCounter,
        destination: ProgramCounter,
        exception: ClassReference,
    ) -> Self {
        Self {
            src: source,
            dst: destination,
            kind: ControlFlowEdgeKind::Exception(exception),
        }
    }

    /// Creates a new subroutine return control flow edge.
    #[must_use]
    pub fn subroutine_return(source: ProgramCounter, destination: ProgramCounter) -> Self {
        Self {
            src: source,
            dst: destination,
            kind: ControlFlowEdgeKind::SubroutineReturn,
        }
    }
}

/// The kind of a [`ControlFlowEdge`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControlFlowEdgeKind {
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
#[derive(Debug, Clone, Default)]
pub struct ControlFlowGraph {
    inner: BTreeMap<(ProgramCounter, ProgramCounter), ControlFlowEdgeKind>,
}

impl ControlFlowGraph {
    pub(crate) fn from_edges(edges: impl IntoIterator<Item = ControlFlowEdge>) -> Self {
        let mut inner = BTreeMap::new();
        edges
            .into_iter()
            .for_each(|ControlFlowEdge { src, dst, kind }| {
                assert!(inner.insert((src, dst), kind).is_none(), "Duplitcate edge");
            });
        Self { inner }
    }
}

pub mod petgraph;
