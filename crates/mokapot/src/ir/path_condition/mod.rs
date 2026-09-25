//! Path condition analysis.

use std::{
    collections::HashMap,
    fmt::Display,
    hash::Hash,
    ops::{BitAnd, BitOr},
};

use itertools::Itertools;

use super::{BlockId, BranchGuard, MokaIRMethod, expression::Predicate};
use crate::analysis::fixed_point::{self, QueuedFactsMap};

mod analyzer;
mod budget;
mod cover;
mod cube;
mod minimizer;

#[cfg(test)]
mod tests;

pub use budget::SolvingBudget;
use cover::Cover;

impl PathCondition<Predicate> {
    /// Computes path conditions at the reachable blocks of `method`.
    ///
    /// Predicates in the result own their operands. Constant SSA definitions
    /// are substituted into edge guards, and decidable guards are folded.
    #[must_use]
    pub fn analyze(method: &MokaIRMethod) -> HashMap<BlockId, Self> {
        Self::analyze_with_budget(method, SolvingBudget::default())
    }

    /// Computes path conditions with a custom minimization budget.
    ///
    /// Intermediate facts skip heuristic generalization. Returned facts are
    /// reduced once with the full budget.
    #[must_use]
    pub fn analyze_with_budget(
        method: &MokaIRMethod,
        budget: SolvingBudget,
    ) -> HashMap<BlockId, Self> {
        let mut problem = analyzer::PathConditionProblem::new(method, budget);
        // Visit blocks in successor order so worklist behavior is stable.
        let Ok(path_conditions): Result<QueuedFactsMap<BlockId, _>, _> =
            fixed_point::solve(&mut problem);
        path_conditions
            .into_iter()
            .map(|(block, fact)| (block, fact.into_inner().reduce_with_budget(budget)))
            .collect()
    }
}

/// A path condition stored in disjunctive normal form.
///
/// Equality compares the stored form, so equivalent conditions may compare unequal.
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

    /// Returns whether this condition is `⊥`.
    #[must_use]
    pub fn is_contradiction(&self) -> bool {
        self.cover.is_contradiction()
    }

    /// Reduces this condition without the heuristic generalization pass.
    ///
    /// Only suitable for intermediate facts: the result is expected to be
    /// reduced again with [`Self::reduce_with_budget`] before it is observed.
    pub(super) fn reduce_without_generalization(self, budget: SolvingBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(self.cover.reduce_without_generalization(budget))
    }

    /// Reduces this condition with the given minimization budget.
    ///
    /// Boolean composition on a [`PathCondition`] does not minimize implicitly.
    #[must_use]
    pub fn reduce_with_budget(self, budget: SolvingBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(self.cover.reduce(budget))
    }
}

impl<P> PathCondition<P> {
    pub(super) fn conjoin_branch_guard(&self, guard: BranchGuard<P>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(self.cover.conjoin_branch_guard(guard))
    }

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

impl<P> BitAnd<BranchGuard<P>> for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: BranchGuard<P>) -> Self::Output {
        self.conjoin_branch_guard(rhs)
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
