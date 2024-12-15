use std::{collections::BTreeMap, convert::Infallible, fmt::Display};

use crate::{
    analysis::fixed_point,
    ir::{self, control_flow::ControlTransfer, ControlFlowGraph, Operand},
    jvm::{code::ProgramCounter, ConstantValue},
};

use super::PathCondition;

/// An analyzer for path conditions.
#[derive(Debug)]
pub struct Analyzer<'a, N> {
    cfg: &'a ControlFlowGraph<N, ControlTransfer>,
}

impl<'a, N> Analyzer<'a, N> {
    /// Creates a new path condition analyzer.
    #[must_use]
    pub fn new(cfg: &'a ControlFlowGraph<N, ControlTransfer>) -> Self {
        Self { cfg }
    }
}

impl<N> fixed_point::Analyzer for Analyzer<'_, N> {
    type Location = ProgramCounter;

    type Fact = PathCondition<Predicate<Value>>;

    type Err = Infallible;

    type AffectedLocations = BTreeMap<Self::Location, Self::Fact>;

    fn entry_fact(&self) -> Result<Self::AffectedLocations, Self::Err> {
        Ok(BTreeMap::from([(
            self.cfg.entry_point(),
            PathCondition::tautology(),
        )]))
    }

    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::AffectedLocations, Self::Err> {
        let Some(outgoing_edges) = self.cfg.edges_from(*location) else {
            return Ok(BTreeMap::default());
        };
        let result = outgoing_edges
            .map(|(_, dst, trx)| match trx {
                ControlTransfer::Conditional(cond) => {
                    let new_cond = cond.clone() & fact.clone();
                    (dst, new_cond)
                }
                _ => (dst, fact.clone()),
            })
            .collect();
        Ok(result)
    }

    fn merge_facts(
        &self,
        current_fact: &Self::Fact,
        incoming_fact: Self::Fact,
    ) -> Result<Self::Fact, Self::Err> {
        let mut result = current_fact.clone() | incoming_fact;
        result.simplify();
        Ok(result)
    }
}

/// A condition.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Predicate<V> {
    /// The left-hand side is equal to the right-hand side.
    Equal(V, V),
    /// The left-hand side is not equal to the right-hand side.
    NotEqual(V, V),
    /// The left-hand side is less than the right-hand side.
    LessThan(V, V),
    /// The left-hand side is less than or equal to the right-hand side.
    LessThanOrEqual(V, V),
    /// The value is null.
    IsNull(V),
    /// The value is not null.
    IsNotNull(V),
}

impl<V> std::ops::Not for Predicate<V> {
    type Output = Self;

    fn not(self) -> Self::Output {
        #[allow(clippy::enum_glob_use)]
        use Predicate::*;
        match self {
            Equal(lhs, rhs) => NotEqual(lhs, rhs),
            NotEqual(lhs, rhs) => Equal(lhs, rhs),
            LessThan(lhs, rhs) => LessThanOrEqual(rhs, lhs),
            LessThanOrEqual(lhs, rhs) => LessThan(rhs, lhs),
            IsNull(value) => IsNotNull(value),
            IsNotNull(value) => IsNull(value),
        }
    }
}

impl<V> Display for Predicate<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal(lhs, rhs) => write!(f, "{lhs} == {rhs}"),
            Self::NotEqual(lhs, rhs) => write!(f, "{lhs} != {rhs}"),
            Self::LessThan(lhs, rhs) => write!(f, "{lhs} < {rhs}"),
            Self::LessThanOrEqual(lhs, rhs) => write!(f, "{lhs} <= {rhs}"),
            Self::IsNull(value) => write!(f, "{value} == null"),
            Self::IsNotNull(value) => write!(f, "{value} != null"),
        }
    }
}

impl<C: Ord> From<Predicate<C>> for PathCondition<Predicate<C>> {
    fn from(value: Predicate<C>) -> Self {
        PathCondition::conjuction_of([value])
    }
}

impl From<ir::expression::Condition> for Predicate<Value> {
    fn from(value: ir::expression::Condition) -> Self {
        #[allow(clippy::enum_glob_use)]
        use ir::expression::Condition::*;

        let zero = ConstantValue::Integer(0).into();
        match value {
            IsZero(value) => Predicate::Equal(value.into(), zero),
            IsNonZero(value) => Predicate::NotEqual(value.into(), zero),
            IsPositive(value) => Predicate::LessThan(zero, value.into()),
            IsNegative(value) => Predicate::LessThan(value.into(), zero),
            IsNonPositive(value) => Predicate::LessThanOrEqual(value.into(), zero),
            IsNonNegative(value) => Predicate::LessThanOrEqual(zero, value.into()),
            Equal(lhs, rhs) => Predicate::Equal(lhs.into(), rhs.into()),
            NotEqual(lhs, rhs) => Predicate::NotEqual(lhs.into(), rhs.into()),
            LessThan(lhs, rhs) => Predicate::LessThan(lhs.into(), rhs.into()),
            LessThanOrEqual(lhs, rhs) => Predicate::LessThanOrEqual(lhs.into(), rhs.into()),
            GreaterThan(lhs, rhs) => Predicate::LessThan(rhs.into(), lhs.into()),
            GreaterThanOrEqual(lhs, rhs) => Predicate::LessThanOrEqual(rhs.into(), lhs.into()),
            IsNull(value) => Predicate::IsNull(value.into()),
            IsNotNull(value) => Predicate::IsNotNull(value.into()),
        }
    }
}

/// A value.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, derive_more::Display)]
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
