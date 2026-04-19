//! Path condition analysis.

use std::{
    collections::HashMap,
    collections::HashSet,
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{BitAnd, BitOr},
};

use crate::{
    analysis::fixed_point,
    ir::{ControlFlowGraph, control_flow::ControlTransfer, expression::Condition},
    jvm::code::ProgramCounter,
};
use itertools::Itertools;

mod analyzer;
mod branch_guard;
mod budget;
mod cover;
mod cube;
mod literal;
mod minimizer;
mod predicate;

#[cfg(test)]
mod tests;

use cover::Cover;

pub use branch_guard::BranchGuard;
pub use budget::PathConditionBudget;
pub use literal::BooleanVariable;
pub use predicate::Value;

pub(super) fn analyze<N>(
    cfg: &ControlFlowGraph<N, ControlTransfer>,
    budget: PathConditionBudget,
) -> HashMap<ProgramCounter, PathCondition<&Condition<Value>>> {
    let mut problem = analyzer::PathConditionProblem::new(cfg, budget);
    let Ok(path_conditions): Result<HashMap<_, _>, _> = fixed_point::solve(&mut problem);
    path_conditions
        .into_iter()
        .map(|(program_counter, fact)| (program_counter, fact.into_inner()))
        .collect()
}

/// A path condition stored in disjunctive normal form.
#[derive(Debug, Clone)]
pub struct PathCondition<P> {
    cover: Cover<P>,
}

impl<P> PartialEq for PathCondition<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.cover == other.cover
    }
}

impl<P> Eq for PathCondition<P> where P: Hash + Eq {}

impl<P> Hash for PathCondition<P>
where
    P: Hash + Eq,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cover.hash(state);
    }
}

impl<P> PathCondition<P> {
    const fn with_cover(cover: Cover<P>) -> Self {
        Self { cover }
    }

    /// Creates the tautological condition `⊤`.
    #[must_use]
    pub fn one() -> Self
    where
        P: Hash + Eq,
    {
        Self::with_cover(Cover::one())
    }

    /// Creates the contradictory condition `⊥`.
    #[must_use]
    pub fn zero() -> Self {
        Self::with_cover(Cover::zero())
    }

    /// Creates a path condition from a single literal.
    #[must_use]
    pub fn of(predicate: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self::with_cover(Cover::of_literal(predicate))
    }

    /// Returns the predicates referenced by this condition.
    #[must_use]
    pub fn predicates(&self) -> HashSet<&P>
    where
        P: Hash + Eq,
    {
        self.cover.predicates().collect()
    }

    /// Returns whether this condition is `⊥`.
    #[must_use]
    pub fn is_contradiction(&self) -> bool {
        self.cover.is_contradiction()
    }

    /// Reduces this condition with a default minimization budget.
    #[must_use]
    pub fn reduce(self) -> Self
    where
        P: Hash + Eq + Clone,
    {
        self.reduce_with_budget(PathConditionBudget::default())
    }

    /// Reduces this condition with the given minimization budget.
    ///
    /// This is an explicit structural optimization step. Raw boolean
    /// composition on [`PathCondition`] does not perform semantic
    /// minimization implicitly.
    #[must_use]
    pub fn reduce_with_budget(self, budget: PathConditionBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(self.cover.reduce(budget))
    }

    #[cfg(test)]
    fn from_branch_guards(branch_guards: impl IntoIterator<Item = BranchGuard<P>>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(Cover::from_branch_guards(branch_guards))
    }

    #[cfg(test)]
    fn cubes(&self) -> impl Iterator<Item = &cube::Cube<P>> {
        self.cover.cubes()
    }
}

impl<P> BitOr for PathCondition<P>
where
    P: Hash + Eq,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::with_cover(self.cover.disjoin(rhs.cover))
    }
}

impl<P> BitAnd<BooleanVariable<P>> for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: BooleanVariable<P>) -> Self::Output {
        Self::with_cover(self.cover.conjoin_literal(&rhs))
    }
}

impl<P> BitAnd<BranchGuard<P>> for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: BranchGuard<P>) -> Self::Output {
        Self::with_cover(self.cover.conjoin_branch_guard(rhs))
    }
}

impl<P> BitAnd for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::with_cover(self.cover.conjoin(&rhs.cover))
    }
}

impl<P> Display for PathCondition<P>
where
    P: Display + Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_contradiction() {
            write!(f, "⊥")
        } else {
            self.cover
                .cubes()
                .map(ToString::to_string)
                .sorted()
                .format(" || ")
                .fmt(f)
        }
    }
}
