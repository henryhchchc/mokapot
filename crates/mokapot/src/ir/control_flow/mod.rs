//! Control-flow analysis.

pub mod path_condition;

use std::{collections::HashMap, hash::Hash};

use self::path_condition::{BranchGuard, PathCondition, SolvingBudget, Value};
use super::{BasicBlock, BlockId, EdgeId, ValueId};
use crate::{
    ir::expression::{Condition, Predicate},
    jvm::references::ClassRef,
};

mod transfer {
    use super::{BranchGuard, ClassRef, Condition, Hash, Value, ValueId};

    /// A state transfer parameterized by the lifting operand representation.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum ControlTransfer<OP: Eq + Hash = ValueId> {
        /// An unconditional control transfer.
        Unconditional,
        /// The normal outcome of a fallible operation.
        Normal,
        /// A conditional transfer guarded by a conjunction of literals.
        Conditional(BranchGuard<Condition<Value<OP>>>),
        /// An exceptional outcome selected by this catch type.
        ///
        /// `None` denotes a catch-all exception-table entry.
        Exception(Option<ClassRef>),
        /// An exceptional outcome that leaves the method.
        Unwind,
    }
}

pub use transfer::ControlTransfer;

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
