//! Control-flow analysis.

pub mod path_condition;

use std::collections::HashMap;

use self::path_condition::BranchGuard;
use super::{BasicBlock, BlockId, EdgeId, Successor};
use crate::{ir::expression::Predicate, jvm::references::ClassRef};

/// The semantics of one control-flow successor arm.
///
/// Exceptional arms begin a new block, so an operation that can raise never
/// coalesces with the location its unguarded arm targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControlTransfer {
    /// An unguarded transfer that always reaches its target.
    Unconditional,
    /// A conditional transfer guarded by a conjunction of literals.
    Conditional(BranchGuard<Predicate>),
    /// An exceptional outcome selected by this catch type.
    ///
    /// `None` denotes a catch-all exception-table entry. Arm order retains
    /// JVM exception-handler precedence.
    Exception(Option<ClassRef>),
}

/// A borrowed edge from a block terminator.
#[derive(Debug, Clone, Copy)]
pub struct Edge<'method> {
    id: EdgeId,
    source: BlockId,
    target: BlockId,
    data: &'method ControlTransfer,
}

impl<'method> Edge<'method> {
    /// Returns this arm's identity.
    #[must_use]
    pub const fn id(self) -> EdgeId {
        self.id
    }

    /// Returns the source block.
    #[must_use]
    pub const fn source(self) -> BlockId {
        self.source
    }

    /// Returns the target block.
    #[must_use]
    pub const fn target(self) -> BlockId {
        self.target
    }

    /// Returns the state transfer associated with this arm.
    #[must_use]
    pub const fn transfer(self) -> &'method ControlTransfer {
        self.data
    }
}

pub(super) fn outgoing_edges(
    blocks: &HashMap<BlockId, BasicBlock>,
    source: BlockId,
) -> impl Iterator<Item = Edge<'_>> {
    blocks.get(&source).into_iter().flat_map(move |block| {
        block
            .terminator
            .successors()
            .filter_map(move |successor| match successor {
                Successor::Block {
                    id,
                    target,
                    transfer,
                    ..
                } => Some(Edge {
                    id: *id,
                    source,
                    target: *target,
                    data: transfer,
                }),
                Successor::Unwind { .. } => None,
            })
    })
}

#[cfg(test)]
mod tests;
