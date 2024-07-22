//! Path constraint analysis.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use crate::{
    analysis::fixed_point,
    ir::{self, control_flow::ControlTransfer, ControlFlowGraph, Operand},
    jvm::{code::ProgramCounter, ConstantValue},
};

/// Path condition in disjunctive normal form.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct DNF<T>(BTreeSet<BTreeSet<T>>);

impl<T> DNF<T>
where
    T: Ord + Clone + std::ops::Not<Output = T>,
{
    fn simplify(&mut self) {
        let mut should_continue = true;
        while should_continue {
            should_continue = false;
            // Remove contradictory clauses.
            self.0.retain(|conjunctive_clause| {
                // A term is contradictory if it contains both a condition and its negation.
                let should_remove = conjunctive_clause
                    .iter()
                    .any(|term| conjunctive_clause.contains(&!term.clone()));
                should_continue = should_continue || should_remove;
                !should_remove
            });
            // Remove redundant clauses.
            let clauses_clone = self.0.clone();
            self.0.retain(|it| {
                // A clause is redundant if it is a supetset of another clause.
                let should_remove = clauses_clone
                    .iter()
                    .any(|other| it.is_superset(other) && it.len() > other.len());
                should_continue = should_continue || should_remove;
                !should_remove
            });
            // Simplify pairs of clauses.
            // A pair of clauses can be simplified if their difference is negation of each other.
            let clone1 = self.0.clone();
            let clone2 = self.0.clone();
            clone1
                .into_iter()
                .flat_map(|l| clone2.clone().into_iter().map(move |r| (l.clone(), r)))
                .for_each(|(mut lhs, mut rhs)| {
                    let remove_from_lhs: BTreeSet<_> = lhs
                        .iter()
                        .filter(|&it| rhs.contains(&!it.clone()))
                        .cloned()
                        .collect();
                    let remove_from_rhs: BTreeSet<_> = rhs
                        .iter()
                        .filter(|&it| rhs.contains(&!it.clone()))
                        .cloned()
                        .collect();
                    for it in &remove_from_lhs {
                        lhs.remove(it);
                    }
                    for it in &remove_from_rhs {
                        rhs.remove(it);
                    }
                    self.0.insert(lhs);
                    self.0.insert(rhs);
                });
        }
    }
}

impl<T> Default for DNF<T> {
    fn default() -> Self {
        Self(BTreeSet::default())
    }
}

impl<T> std::ops::BitOr for DNF<T>
where
    T: Ord + Clone + std::ops::Not<Output = T>,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let DNF(lhs) = self;
        let DNF(rhs) = rhs;
        if lhs.is_empty() || rhs.is_empty() {
            DNF::default()
        } else {
            let clauses: BTreeSet<_> = lhs.into_iter().chain(rhs).collect();
            let mut result = DNF(clauses);
            result.simplify();
            result
        }
    }
}

impl<T> std::ops::BitAnd for DNF<T>
where
    T: Ord + Clone + std::ops::Not<Output = T>,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let DNF(this) = self;
        let DNF(other) = rhs;
        if this.is_empty() {
            DNF(other)
        } else if other.is_empty() {
            DNF(this)
        } else {
            let clauses = this
                .into_iter()
                .flat_map(|lhs_con| {
                    other.clone().into_iter().map(move |rhs_con| {
                        lhs_con.clone().into_iter().chain(rhs_con.clone()).collect()
                    })
                })
                .collect();
            let mut result = DNF(clauses);
            result.simplify();
            result
        }
    }
}

impl<T> std::ops::Not for DNF<T>
where
    T: Ord + Clone + std::ops::Not<Output = T>,
{
    type Output = Self;

    fn not(self) -> Self::Output {
        let DNF(clauses) = self;
        let clauses = BTreeSet::from([clauses.into_iter().flatten().map(|it| !it).collect()]);
        let mut result = DNF(clauses);
        result.simplify();
        result
    }
}

impl<T> Display for DNF<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let DNF(clauses) = self;
        for (i, conj) in clauses.iter().enumerate() {
            if i > 0 {
                write!(f, " || ")?;
            }
            if conj.len() > 1 {
                write!(f, "(")?;
            }
            for (j, cond) in conj.iter().enumerate() {
                if j > 0 {
                    write!(f, " && ")?;
                }
                write!(f, "{cond}")?;
            }
            if conj.len() > 1 {
                write!(f, ")")?;
            }
        }

        Ok(())
    }
}

/// A condition.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Condition<V> {
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

impl<V> std::ops::Not for Condition<V> {
    type Output = Self;

    fn not(self) -> Self::Output {
        #[allow(clippy::enum_glob_use)]
        use Condition::*;
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

impl<V> Display for Condition<V>
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

impl<C: Ord> From<Condition<C>> for DNF<Condition<C>> {
    fn from(value: Condition<C>) -> Self {
        DNF(BTreeSet::from([BTreeSet::from([value])]))
    }
}

impl From<ir::expression::Condition> for DNF<Condition<Value>> {
    fn from(value: ir::expression::Condition) -> Self {
        #[allow(clippy::enum_glob_use)]
        use ir::expression::Condition::*;

        let zero = ConstantValue::Integer(0).into();
        let cond = match value {
            IsZero(value) => Condition::Equal(value.into(), zero),
            IsNonZero(value) => Condition::NotEqual(value.into(), zero),
            IsPositive(value) => Condition::LessThan(zero, value.into()),
            IsNegative(value) => Condition::LessThan(value.into(), zero),
            IsNonPositive(value) => Condition::LessThanOrEqual(value.into(), zero),
            IsNonNegative(value) => Condition::LessThanOrEqual(zero, value.into()),
            Equal(lhs, rhs) => Condition::Equal(lhs.into(), rhs.into()),
            NotEqual(lhs, rhs) => Condition::NotEqual(lhs.into(), rhs.into()),
            LessThan(lhs, rhs) => Condition::LessThan(lhs.into(), rhs.into()),
            LessThanOrEqual(lhs, rhs) => Condition::LessThanOrEqual(lhs.into(), rhs.into()),
            GreaterThan(lhs, rhs) => Condition::LessThan(rhs.into(), lhs.into()),
            GreaterThanOrEqual(lhs, rhs) => Condition::LessThanOrEqual(rhs.into(), lhs.into()),
            IsNull(value) => Condition::IsNull(value.into()),
            IsNotNull(value) => Condition::IsNotNull(value.into()),
        };
        cond.into()
    }
}

/// A value.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, derive_more::Display)]
pub enum Value {
    /// An operand.
    Operand(Operand),
    /// A constant value.
    Constant(ConstantValue),
}

impl From<ir::Operand> for Value {
    fn from(value: ir::Operand) -> Self {
        Self::Operand(value)
    }
}

impl From<ConstantValue> for Value {
    fn from(value: ConstantValue) -> Self {
        Self::Constant(value)
    }
}

/// An analyzer for path conditions.
#[derive(Debug)]
pub struct Analyzer<'a> {
    cfg: &'a ControlFlowGraph<(), ControlTransfer>,
}

impl<'a> Analyzer<'a> {
    /// Creates a new path condition analyzer.
    #[must_use]
    pub fn new(cfg: &'a ControlFlowGraph<(), ControlTransfer>) -> Self {
        Self { cfg }
    }
}

impl fixed_point::Analyzer for Analyzer<'_> {
    type Location = ProgramCounter;

    type Fact = DNF<Condition<Value>>;

    type Err = ();

    type AffectedLocations = BTreeMap<Self::Location, Self::Fact>;

    fn entry_fact(&self) -> Result<Self::AffectedLocations, Self::Err> {
        Ok(BTreeMap::from([(
            ProgramCounter::ZERO,
            Self::Fact::default(),
        )]))
    }

    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::AffectedLocations, Self::Err> {
        Ok(self
            .cfg
            .edges_from(*location)
            .into_iter()
            .flatten()
            .map(|(_, dst, trx)| match trx {
                ControlTransfer::Conditional(cond) => (dst, cond.clone() & fact.clone()),
                _ => (dst, fact.clone()),
            })
            .collect())
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

#[cfg(test)]
mod test {}
