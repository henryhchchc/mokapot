use std::{collections::BTreeMap, convert::Infallible, fmt::Display};

use crate::{
    analysis::fixed_point,
    ir::{self, ControlFlowGraph, Operand, control_flow::ControlTransfer},
    jvm::{ConstantValue, code::ProgramCounter},
};

use super::{MinTerm, SOP, Variable};

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

    type Fact = SOP<Predicate<Value>>;

    type Err = Infallible;

    type AffectedLocations = BTreeMap<Self::Location, Self::Fact>;

    fn entry_fact(&self) -> Result<Self::AffectedLocations, Self::Err> {
        Ok(BTreeMap::from([(self.cfg.entry_point(), SOP::one())]))
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
                    let mut cond = cond.clone();
                    cond.simplify();
                    let mut new_cond = cond & fact.clone();
                    new_cond.simplify();
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
    /// The left-hand side is less than the right-hand side.
    LessThan(V, V),
    /// The left-hand side is less than or equal to the right-hand side.
    LessThanOrEqual(V, V),
    /// The value is null.
    IsNull(V),
}

impl<V> Display for Predicate<V>
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

impl From<ir::expression::Condition> for SOP<Predicate<Value>> {
    fn from(value: ir::expression::Condition) -> Self {
        Self::from_iter([MinTerm::from_iter([value.into()])])
    }
}

impl From<ir::expression::Condition> for Variable<Predicate<Value>> {
    fn from(value: ir::expression::Condition) -> Self {
        #[allow(clippy::enum_glob_use)]
        use ir::expression::Condition::*;

        let zero = ConstantValue::Integer(0).into();
        match value {
            IsZero(value) => Variable::Positive(Predicate::Equal(value.into(), zero)),
            IsNonZero(value) => Variable::Negative(Predicate::Equal(value.into(), zero)),
            IsPositive(value) => Variable::Negative(Predicate::LessThanOrEqual(value.into(), zero)),
            IsNegative(value) => Variable::Positive(Predicate::LessThan(value.into(), zero)),
            IsNonPositive(value) => {
                Variable::Positive(Predicate::LessThanOrEqual(value.into(), zero))
            }
            IsNonNegative(value) => {
                Variable::Positive(Predicate::LessThanOrEqual(zero, value.into()))
            }
            Equal(lhs, rhs) => Variable::Positive(Predicate::Equal(lhs.into(), rhs.into())),
            NotEqual(lhs, rhs) => Variable::Negative(Predicate::Equal(lhs.into(), rhs.into())),
            LessThan(lhs, rhs) => Variable::Positive(Predicate::LessThan(lhs.into(), rhs.into())),
            LessThanOrEqual(lhs, rhs) => {
                Variable::Positive(Predicate::LessThanOrEqual(lhs.into(), rhs.into()))
            }
            GreaterThan(lhs, rhs) => {
                Variable::Negative(Predicate::LessThanOrEqual(lhs.into(), rhs.into()))
            }
            GreaterThanOrEqual(lhs, rhs) => {
                Variable::Negative(Predicate::LessThan(lhs.into(), rhs.into()))
            }
            IsNull(value) => Variable::Positive(Predicate::IsNull(value.into())),
            IsNotNull(value) => Variable::Negative(Predicate::IsNull(value.into())),
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
