use std::convert::Infallible;

use super::{BooleanVariable, PathCondition};
use crate::{
    analysis::fixed_point::DataflowProblem,
    ir::{
        self, ControlFlowGraph, Operand,
        control_flow::{ControlTransfer, Edge},
        expression::Condition,
    },
    jvm::{ConstantValue, code::ProgramCounter},
};

/// An analyzer for path conditions.
#[derive(Debug)]
pub struct Analyzer<'a, N> {
    cfg: &'a ControlFlowGraph<N, ControlTransfer>,
}

impl<'a, N> Analyzer<'a, N> {
    /// Creates a new path condition analyzer.
    #[must_use]
    pub const fn new(cfg: &'a ControlFlowGraph<N, ControlTransfer>) -> Self {
        Self { cfg }
    }
}

impl<'cfg, N> DataflowProblem for Analyzer<'cfg, N> {
    type Location = ProgramCounter;

    type Fact = PathCondition<&'cfg Condition<Value>>;

    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(self.cfg.entry_point(), PathCondition::one())]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        Ok(self
            .cfg
            .outgoing_edges(*location)
            .into_iter()
            .flatten()
            .map(|edge| match edge.data {
                ControlTransfer::Conditional(cond) => (edge.target, cond.as_ref() & fact.clone()),
                _ => (edge.target, fact.clone()),
            })
            .collect::<Vec<_>>())
    }
}

impl<T, V> From<ir::expression::Condition<T>> for BooleanVariable<ir::expression::Condition<V>>
where
    V: From<T>,
{
    fn from(value: ir::expression::Condition<T>) -> Self {
        #[allow(clippy::enum_glob_use)]
        use Condition::*;
        let inner = match value {
            Equal(lhs, rhs) => Equal(lhs.into(), rhs.into()),
            NotEqual(lhs, rhs) => NotEqual(lhs.into(), rhs.into()),
            LessThan(lhs, rhs) => LessThan(lhs.into(), rhs.into()),
            LessThanOrEqual(lhs, rhs) => LessThanOrEqual(lhs.into(), rhs.into()),
            GreaterThan(lhs, rhs) => GreaterThan(lhs.into(), rhs.into()),
            GreaterThanOrEqual(lhs, rhs) => GreaterThanOrEqual(lhs.into(), rhs.into()),
            IsNull(value) => IsNull(value.into()),
            IsNotNull(value) => IsNotNull(value.into()),
            IsZero(value) => IsZero(value.into()),
            IsNonZero(value) => IsNonZero(value.into()),
            IsPositive(value) => IsPositive(value.into()),
            IsNegative(value) => IsNegative(value.into()),
            IsNonNegative(value) => IsNonNegative(value.into()),
            IsNonPositive(value) => IsNonPositive(value.into()),
        };
        Self::Positive(inner)
    }
}

/// A value.
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, derive_more::Display)]
pub enum Value {
    /// A variable.
    Variable(Operand),
    /// A constant value.
    Constant(ConstantValue),
}

impl From<ir::Operand> for Value {
    fn from(value: ir::Operand) -> Self {
        Self::Variable(value)
    }
}

impl From<ConstantValue> for Value {
    fn from(value: ConstantValue) -> Self {
        Self::Constant(value)
    }
}
