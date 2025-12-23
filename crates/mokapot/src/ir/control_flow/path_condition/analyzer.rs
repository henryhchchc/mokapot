use std::{convert::Infallible, fmt::Display};

use super::{BooleanVariable, PathCondition};
use crate::{
    analysis::fixed_point::DataflowProblem,
    ir::{self, ControlFlowGraph, Operand, control_flow::ControlTransfer},
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

    type Fact = PathCondition<&'cfg NormalizedPredicate<Value>>;

    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(self.cfg.entry_point(), PathCondition::one())]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        let Some(outgoing_edges) = self.cfg.outgoing_edges(*location) else {
            return Ok(Vec::new());
        };
        let result: Vec<_> = outgoing_edges
            .map(|edge| {
                let new_fact = if let ControlTransfer::Conditional(cond) = edge.data {
                    cond.as_ref() & fact.clone()
                } else {
                    fact.clone()
                };
                (edge.target, new_fact)
            })
            .collect();
        Ok(result)
    }
}

/// A normalized condition.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum NormalizedPredicate<V> {
    /// The left-hand side is equal to the right-hand side.
    Equal(V, V),
    /// The left-hand side is less than the right-hand side.
    LessThan(V, V),
    /// The left-hand side is less than or equal to the right-hand side.
    LessThanOrEqual(V, V),
    /// The value is null.
    IsNull(V),
}

impl<V> Display for NormalizedPredicate<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal(lhs, rhs) => write!(f, "{lhs} == {rhs}"),
            Self::LessThan(lhs, rhs) => write!(f, "{lhs} < {rhs}"),
            Self::LessThanOrEqual(lhs, rhs) => write!(f, "{lhs} <= {rhs}"),
            Self::IsNull(value) => write!(f, "{value} == null"),
        }
    }
}

impl From<ir::expression::Condition> for BooleanVariable<NormalizedPredicate<Value>> {
    fn from(value: ir::expression::Condition) -> Self {
        use BooleanVariable::{Negative, Positive};
        #[allow(clippy::enum_glob_use)]
        use ir::expression::Condition::*;

        let zero: Value = ConstantValue::Integer(0).into();
        match value {
            IsNull(value) => Positive(NormalizedPredicate::IsNull(value.into())),
            IsNotNull(value) => Negative(NormalizedPredicate::IsNull(value.into())),
            // For binary operation involving zeros, we always put zero on the right.
            IsZero(value) => Positive(NormalizedPredicate::Equal(value.into(), zero)),
            IsNonZero(value) => Negative(NormalizedPredicate::Equal(value.into(), zero)),
            IsPositive(value) => Negative(NormalizedPredicate::LessThanOrEqual(value.into(), zero)),
            IsNegative(value) => Positive(NormalizedPredicate::LessThan(value.into(), zero)),
            IsNonPositive(value) => {
                Positive(NormalizedPredicate::LessThanOrEqual(value.into(), zero))
            }
            IsNonNegative(value) => Negative(NormalizedPredicate::LessThan(value.into(), zero)),
            // For binary operations, we establish a normalized form
            // by placing the smaller value on the left-hand side.
            Equal(lhs, rhs) if lhs < rhs => {
                Positive(NormalizedPredicate::Equal(lhs.into(), rhs.into()))
            }
            Equal(lhs, rhs) => Positive(NormalizedPredicate::Equal(rhs.into(), lhs.into())),
            NotEqual(lhs, rhs) if lhs < rhs => {
                Negative(NormalizedPredicate::Equal(lhs.into(), rhs.into()))
            }
            NotEqual(lhs, rhs) => Negative(NormalizedPredicate::Equal(rhs.into(), lhs.into())),
            LessThan(lhs, rhs) if lhs < rhs => {
                Positive(NormalizedPredicate::LessThan(lhs.into(), rhs.into()))
            }
            LessThan(lhs, rhs) => {
                Negative(NormalizedPredicate::LessThanOrEqual(rhs.into(), lhs.into()))
            }
            LessThanOrEqual(lhs, rhs) if lhs < rhs => {
                Positive(NormalizedPredicate::LessThanOrEqual(lhs.into(), rhs.into()))
            }
            LessThanOrEqual(lhs, rhs) => {
                Negative(NormalizedPredicate::LessThan(rhs.into(), lhs.into()))
            }
            GreaterThan(lhs, rhs) if lhs < rhs => {
                Negative(NormalizedPredicate::LessThanOrEqual(lhs.into(), rhs.into()))
            }
            GreaterThan(lhs, rhs) => {
                Positive(NormalizedPredicate::LessThan(rhs.into(), lhs.into()))
            }
            GreaterThanOrEqual(lhs, rhs) if lhs < rhs => {
                Negative(NormalizedPredicate::LessThan(lhs.into(), rhs.into()))
            }
            GreaterThanOrEqual(lhs, rhs) => {
                Positive(NormalizedPredicate::LessThanOrEqual(rhs.into(), lhs.into()))
            }
        }
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
