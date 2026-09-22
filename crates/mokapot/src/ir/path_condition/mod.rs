//! Path condition analysis.

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{BitAnd, BitOr},
};

use itertools::Itertools;

use crate::{
    analysis::fixed_point,
    ir::{
        BasicBlock, BlockId, BranchGuard, MokaIRMethod,
        expression::{BooleanVariable, Predicate},
    },
};

mod analyzer;
mod budget;
mod cover;
mod cube;
mod minimizer;

#[cfg(test)]
mod tests;

pub use budget::SolvingBudget;
use cover::Cover;

impl<'method> PathCondition<&'method Predicate> {
    /// Computes path conditions at the reachable blocks of `method`.
    #[must_use]
    pub fn analyze(method: &'method MokaIRMethod) -> HashMap<BlockId, Self> {
        Self::analyze_with_budget(method, SolvingBudget::default())
    }

    /// Computes path conditions with a custom minimization budget.
    #[must_use]
    pub fn analyze_with_budget(
        method: &'method MokaIRMethod,
        budget: SolvingBudget,
    ) -> HashMap<BlockId, Self> {
        analyze_blocks(&method.blocks, method.entry_block(), budget)
    }
}

fn analyze_blocks(
    blocks: &HashMap<BlockId, BasicBlock>,
    entry: BlockId,
    budget: SolvingBudget,
) -> HashMap<BlockId, PathCondition<&Predicate>> {
    let mut problem = analyzer::PathConditionProblem::new(blocks, entry, budget);
    let Ok(path_conditions): Result<HashMap<_, _>, _> = fixed_point::solve(&mut problem);
    path_conditions
        .into_iter()
        .map(|(block, fact)| (block, fact.into_inner()))
        .collect()
}

/// A path condition stored in disjunctive normal form.
#[derive(Debug, Clone)]
pub struct PathCondition<P> {
    cover: Cover<P>,
}

/// A borrowed conjunction in a [`PathCondition`] disjunctive normal form.
///
/// A condition is the disjunction of its [`PathCondition::disjuncts`].
#[derive(Debug, Clone, Copy)]
pub struct PathConditionTerm<'a, P>(&'a cube::Cube<P>);

impl<P> PathConditionTerm<'_, P> {
    /// Returns whether this term is the tautological conjunction `⊤`.
    #[must_use]
    pub fn is_tautology(&self) -> bool {
        self.0.is_tautology()
    }
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

    /// Iterates over the conjunctions that this condition disjoins.
    ///
    /// The iteration order is unspecified. A contradictory condition has no
    /// disjuncts, while a tautological condition has one tautological disjunct.
    pub fn disjuncts(&self) -> impl Iterator<Item = PathConditionTerm<'_, P>> {
        self.cover.cubes().map(PathConditionTerm)
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
        self.reduce_with_budget(SolvingBudget::default())
    }

    /// Reduces this condition with the given minimization budget.
    ///
    /// This is an explicit structural optimization step. Raw boolean
    /// composition on [`PathCondition`] does not perform semantic
    /// minimization implicitly.
    #[must_use]
    pub fn reduce_with_budget(self, budget: SolvingBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(self.cover.reduce(budget))
    }
}

impl<P> PathCondition<P> {
    const fn with_cover(cover: Cover<P>) -> Self {
        Self { cover }
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
