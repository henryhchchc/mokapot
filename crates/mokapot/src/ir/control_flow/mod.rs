//! Control-flow analysis.

pub mod path_condition;

use std::collections::HashMap;

use self::path_condition::{BranchGuard, PathCondition, SolvingBudget};
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

/// A borrowed control-flow graph derived solely from block terminators.
#[derive(Debug, Clone, Copy)]
pub struct ControlFlowGraph<'method> {
    pub(super) blocks: &'method HashMap<BlockId, BasicBlock>,
    entry: BlockId,
}

impl<'m> ControlFlowGraph<'m> {
    /// Returns the entry block.
    #[must_use]
    pub const fn entry_block(self) -> BlockId {
        self.entry
    }

    /// Returns all outgoing arms from `source`.
    ///
    /// An identity outside this graph yields no arms.
    pub fn outgoing_edges(self, source: BlockId) -> impl Iterator<Item = Edge<'m>> {
        self.blocks.get(&source).into_iter().flat_map(move |block| {
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

    /// Computes path conditions at reachable blocks.
    #[must_use]
    pub fn path_conditions(self) -> HashMap<BlockId, PathCondition<&'m Predicate>> {
        self.path_conditions_with_budget(SolvingBudget::default())
    }

    /// Computes path conditions with a custom minimization budget.
    #[must_use]
    pub fn path_conditions_with_budget(
        self,
        budget: SolvingBudget,
    ) -> HashMap<BlockId, PathCondition<&'m Predicate>> {
        path_condition::analyze(self, budget)
    }
}

impl<'m> ControlFlowGraph<'m> {
    pub(super) const fn new(blocks: &'m HashMap<BlockId, BasicBlock>, entry: BlockId) -> Self {
        Self { blocks, entry }
    }
}

#[cfg(test)]
mod tests;
