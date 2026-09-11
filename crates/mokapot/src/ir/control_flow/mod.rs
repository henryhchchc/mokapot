//! Control-flow analysis.

pub mod path_condition;

use std::{collections::HashMap, hash::Hash};

use self::path_condition::{BranchGuard, PathCondition, SolvingBudget, Value};
use super::{BasicBlock, BlockId, EdgeId, TryMapValues, ValueId};
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

impl<OP, OUT> TryMapValues<OUT> for ControlTransfer<OP>
where
    OP: Eq + Hash,
    OUT: Eq + Hash,
{
    type Value = OP;
    type Mapped = ControlTransfer<OUT>;

    fn try_map_values<E>(
        self,
        mut remap: impl FnMut(OP) -> Result<OUT, E>,
    ) -> Result<ControlTransfer<OUT>, E> {
        Ok(match self {
            Self::Unconditional => ControlTransfer::Unconditional,
            Self::Normal => ControlTransfer::Normal,
            Self::Exception(exception) => ControlTransfer::Exception(exception),
            Self::Unwind => ControlTransfer::Unwind,
            Self::Conditional(guard) => ControlTransfer::Conditional(
                guard.try_map_values(|value| value.try_map_values(&mut remap))?,
            ),
        })
    }
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

#[cfg(test)]
mod mapping_tests {
    use super::{
        ControlTransfer, TryMapValues,
        path_condition::{BooleanVariable, BranchGuard, Value},
    };
    use crate::{ir::expression::Condition, jvm::ConstantValue};

    #[test]
    fn maps_guard_variables_without_changing_constants_or_polarity() {
        let transfer = ControlTransfer::Conditional(BranchGuard::from_iter([
            BooleanVariable::Positive(Condition::Equal(
                Value::Variable(1_u8),
                Value::Constant(ConstantValue::Integer(3)),
            )),
            BooleanVariable::Negative(Condition::IsNull(Value::Variable(2))),
        ]));

        let mapped = transfer
            .try_map_values(|value| Ok::<_, ()>(u16::from(value) + 10))
            .unwrap();
        assert_eq!(
            mapped,
            ControlTransfer::Conditional(BranchGuard::from_iter([
                BooleanVariable::Positive(Condition::Equal(
                    Value::Variable(11_u16),
                    Value::Constant(ConstantValue::Integer(3))
                )),
                BooleanVariable::Negative(Condition::IsNull(Value::Variable(12))),
            ]))
        );
    }

    #[test]
    fn mapping_errors_propagate_from_guards() {
        let transfer = ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(
            Condition::IsNull(Value::Variable(1_u8)),
        )));
        assert_eq!(
            transfer.try_map_values(|_| Err::<u16, _>("unmapped")),
            Err("unmapped")
        );
    }

    #[test]
    fn mapping_preserves_guard_set_semantics() {
        let transfer = ControlTransfer::Conditional(BranchGuard::from_iter([
            BooleanVariable::Positive(Condition::IsNull(Value::Variable(1_u8))),
            BooleanVariable::Positive(Condition::IsNull(Value::Variable(2))),
        ]));

        let mapped = transfer.try_map_values(|_| Ok::<_, ()>(0_u16)).unwrap();
        assert_eq!(
            mapped,
            ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(
                Condition::IsNull(Value::Variable(0_u16)),
            )))
        );
    }
}
