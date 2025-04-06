use std::{collections::BTreeMap, convert::Infallible, fmt::Display};

use crate::{
    analysis::fixed_point,
    ir::{self, ControlFlowGraph, Operand, control_flow::ControlTransfer},
    jvm::{ConstantValue, code::ProgramCounter},
};

use super::{BooleanVariable, MinTerm, PathCondition};

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

    type Fact = PathCondition<NormalizedPredicate<Value>>;

    type Err = Infallible;

    type AffectedLocations = BTreeMap<Self::Location, Self::Fact>;

    fn entry_fact(&self) -> Result<Self::AffectedLocations, Self::Err> {
        Ok(BTreeMap::from([(
            self.cfg.entry_point(),
            PathCondition::one(),
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
                    let cond = cond.clone();
                    let new_cond = cond & fact.clone();
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
        let result = current_fact.clone() | incoming_fact;
        Ok(result)
    }
}

/// A normalized condition.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
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

impl From<ir::expression::Condition> for PathCondition<NormalizedPredicate<Value>> {
    fn from(value: ir::expression::Condition) -> Self {
        Self::from_iter([MinTerm::from_iter([value.into()])])
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
