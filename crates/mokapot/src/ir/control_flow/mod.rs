//! Control-flow analysis.

pub mod path_condition;

use std::{collections::HashMap, hash::Hash};

use self::path_condition::{BranchGuard, LiftedValue, PathCondition, SolvingBudget};
use super::{BasicBlock, BlockId, EdgeId, ValueId};
use crate::{
    ir::expression::{LiftedCondition, Predicate},
    jvm::references::ClassRef,
};

mod transfer {
    use super::{BranchGuard, ClassRef, Hash, LiftedCondition, LiftedValue};

    /// A state transfer parameterized by the lifting operand representation.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum ControlTransfer<OP: Eq + Hash> {
        /// An unconditional control transfer.
        Unconditional,
        /// The normal outcome of a fallible operation.
        Normal,
        /// A conditional transfer guarded by a conjunction of literals.
        Conditional(BranchGuard<LiftedCondition<LiftedValue<OP>>>),
        /// An exceptional outcome selected by this catch type.
        ///
        /// `None` denotes a catch-all exception-table entry.
        Exception(Option<ClassRef>),
        /// An exceptional outcome that leaves the method.
        Unwind,
    }
}

/// The state transfer associated with one control-flow arm.
pub type ControlTransfer = transfer::ControlTransfer<ValueId>;
pub(crate) use transfer::ControlTransfer as LiftedControlTransfer;

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
mod tests {
    use super::*;
    use crate::ir::{
        BasicBlock, EdgeId, InstructionId, Successor, Terminator, TerminatorKind,
        control_flow::path_condition::BooleanVariable, expression::Condition,
    };

    fn block(id: u32, successors: Vec<Successor>) -> BasicBlock {
        BasicBlock::new(
            BlockId::new(id),
            vec![],
            vec![],
            Terminator::new(
                InstructionId::new(id),
                if successors.len() == 2 {
                    TerminatorKind::Branch
                } else if successors.is_empty() {
                    TerminatorKind::Return(None)
                } else {
                    TerminatorKind::Goto
                },
                successors,
            ),
        )
    }

    #[test]
    fn path_conditions_prune_contradictory_arms_at_block_locations() {
        let condition = Condition::IsZero(crate::ir::ValueId::new(0));
        let positive: BooleanVariable<Predicate> = condition.into();
        let negative = !positive.clone();
        let blocks = vec![
            block(
                0,
                vec![Successor::new(
                    EdgeId::new(0),
                    BlockId::new(1),
                    ControlTransfer::Conditional(BranchGuard::of(positive.clone())),
                )],
            ),
            block(
                1,
                vec![
                    Successor::new(
                        EdgeId::new(1),
                        BlockId::new(2),
                        ControlTransfer::Conditional(BranchGuard::of(negative)),
                    ),
                    Successor::new(
                        EdgeId::new(2),
                        BlockId::new(3),
                        ControlTransfer::Unconditional,
                    ),
                ],
            ),
            block(2, vec![]),
            block(3, vec![]),
        ];
        let conditions = ControlFlowGraph::new(&blocks, BlockId::new(0)).path_conditions();

        assert!(conditions.contains_key(&BlockId::new(0)));
        assert!(conditions.contains_key(&BlockId::new(1)));
        assert!(!conditions.contains_key(&BlockId::new(2)));
        assert!(conditions.contains_key(&BlockId::new(3)));
    }

    #[test]
    fn exceptional_outcomes_preserve_the_incoming_path_condition() {
        let condition = Condition::IsZero(crate::ir::ValueId::new(0));
        let positive: BooleanVariable<Predicate> = condition.into();
        let negative = !positive.clone();
        let blocks = vec![
            block(
                0,
                vec![
                    Successor::new(
                        EdgeId::new(0),
                        BlockId::new(1),
                        ControlTransfer::Conditional(BranchGuard::of(positive)),
                    ),
                    Successor::new(
                        EdgeId::new(1),
                        BlockId::new(5),
                        ControlTransfer::Conditional(BranchGuard::of(negative)),
                    ),
                ],
            ),
            block(
                1,
                vec![
                    Successor::new(EdgeId::new(2), BlockId::new(2), ControlTransfer::Normal),
                    Successor::new(
                        EdgeId::new(3),
                        BlockId::new(3),
                        ControlTransfer::Exception(Some(
                            "java/lang/RuntimeException".parse().unwrap(),
                        )),
                    ),
                    Successor::new(EdgeId::new(4), BlockId::new(4), ControlTransfer::Unwind),
                ],
            ),
            block(2, vec![]),
            block(3, vec![]),
            block(4, vec![]),
            block(5, vec![]),
        ];
        let conditions = ControlFlowGraph::new(&blocks, BlockId::new(0)).path_conditions();

        assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(2)]);
        assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(3)]);
        assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(4)]);
        assert_ne!(conditions[&BlockId::new(1)], conditions[&BlockId::new(5)]);
    }
}
