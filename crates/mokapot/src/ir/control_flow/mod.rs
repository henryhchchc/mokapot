//! Control-flow analysis.

pub mod path_condition;

use std::collections::HashMap;

use self::path_condition::{BranchGuard, PathCondition, SolvingBudget};
use super::{BasicBlock, BlockId, EdgeId};
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
    /// An exceptional outcome that leaves the method.
    Unwind,
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
    pub(crate) blocks: &'method [BasicBlock],
    entry: BlockId,
}

impl<'method> ControlFlowGraph<'method> {
    pub(crate) const fn new(blocks: &'method [BasicBlock], entry: BlockId) -> Self {
        Self { blocks, entry }
    }

    /// Returns the entry block.
    #[must_use]
    pub const fn entry_block(self) -> BlockId {
        self.entry
    }

    /// Returns the blocks in deterministic source order.
    ///
    /// Identities are dense and ascending here, so a block identity doubles as
    /// a position in this sequence.
    #[must_use]
    pub fn nodes(self) -> impl ExactSizeIterator<Item = (BlockId, &'method BasicBlock)> {
        self.blocks.iter().map(|block| (block.id(), block))
    }

    /// Returns every successor arm, retaining parallel edges.
    pub fn edges(self) -> impl Iterator<Item = Edge<'method>> {
        self.blocks.iter().flat_map(|block| {
            block
                .terminator()
                .successors()
                .iter()
                .map(move |successor| Edge {
                    id: successor.id(),
                    source: block.id(),
                    target: successor.target(),
                    data: successor.transfer(),
                })
        })
    }

    /// Returns blocks with no outgoing successor arms.
    pub fn exits(self) -> impl Iterator<Item = BlockId> + 'method {
        self.blocks
            .iter()
            .filter(|block| block.terminator().successors().is_empty())
            .map(BasicBlock::id)
    }

    /// Returns all outgoing arms from `source`.
    ///
    /// The identity is resolved as a position, which relies on the dense,
    /// ascending identities that [`ControlFlowGraph::nodes`] reports; an
    /// identity outside this graph yields no arms.
    pub fn outgoing_edges(self, source: BlockId) -> impl Iterator<Item = Edge<'method>> {
        self.blocks
            .get(usize::try_from(source.index()).unwrap_or(usize::MAX))
            .into_iter()
            .flat_map(move |block| {
                block
                    .terminator()
                    .successors()
                    .iter()
                    .map(move |successor| Edge {
                        id: successor.id(),
                        source,
                        target: successor.target(),
                        data: successor.transfer(),
                    })
            })
    }

    /// Computes path conditions at reachable blocks.
    #[must_use]
    pub fn path_conditions(self) -> HashMap<BlockId, PathCondition<&'method Predicate>> {
        self.path_conditions_with_budget(SolvingBudget::default())
    }

    /// Computes path conditions with a custom minimization budget.
    #[must_use]
    pub fn path_conditions_with_budget(
        self,
        budget: SolvingBudget,
    ) -> HashMap<BlockId, PathCondition<&'method Predicate>> {
        path_condition::analyze(self, budget)
    }
}

#[cfg(test)]
mod tests;
