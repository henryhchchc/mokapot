use std::{cmp, convert::Infallible, hash::Hash};

use super::{BlockId, BranchGuard, MokaIRMethod, PathCondition, Predicate, SolvingBudget};
use crate::{
    analysis::fixed_point::{DataflowProblem, JoinSemiLattice},
    ir::ControlTransfer,
};

/// A forward dataflow analysis that propagates path conditions through a CFG.
#[derive(Debug)]
pub(super) struct PathConditionProblem<'method> {
    method: &'method MokaIRMethod,
    budget: SolvingBudget,
}

impl<'method> PathConditionProblem<'method> {
    /// Creates a path-condition analysis over the given method.
    #[must_use]
    pub(super) const fn new(method: &'method MokaIRMethod, budget: SolvingBudget) -> Self {
        Self { method, budget }
    }
}

impl<'method> DataflowProblem for PathConditionProblem<'method> {
    type Location = BlockId;

    type Fact = PathConditionFact<&'method Predicate>;

    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(
            self.method.entry_block(),
            PathConditionFact::one(self.budget),
        )]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        let block = self
            .method
            .block(*location)
            .expect("a dataflow location is a block of the method");
        Ok(block.terminator.successors().filter_map(|successor| {
            let target = successor.block_target()?;
            let propagated = match successor.transfer() {
                Some(ControlTransfer::Conditional(guard)) => {
                    fact.conjoin_branch_guard(guard.as_ref())
                }
                Some(ControlTransfer::Unconditional | ControlTransfer::Exception(_)) | None => {
                    fact.clone()
                }
            };
            (!propagated.is_contradiction()).then_some((target, propagated))
        }))
    }
}

/// Internal lattice wrapper used by the generic fixed-point solver.
///
/// Facts are reduced without generalization; the returned ones are generalized
/// by [`PathCondition::analyze_with_budget`].
#[derive(Debug, Clone)]
#[doc(hidden)]
pub(super) struct PathConditionFact<P> {
    inner: PathCondition<P>,
    budget: SolvingBudget,
}

impl<P> PathConditionFact<P> {
    pub(super) fn one(budget: SolvingBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::new(PathCondition::one(), budget)
    }

    pub(super) fn new(inner: PathCondition<P>, budget: SolvingBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let inner = inner.reduce_without_generalization(budget);
        Self { inner, budget }
    }

    pub(super) fn conjoin_branch_guard(&self, branch_guard: BranchGuard<P>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::new(self.inner.clone() & branch_guard, self.budget)
    }

    pub(super) fn is_contradiction(&self) -> bool {
        self.inner.is_contradiction()
    }

    pub(super) fn into_inner(self) -> PathCondition<P> {
        self.inner
    }
}

impl<P> PartialEq for PathConditionFact<P>
where
    P: Hash + Eq + Clone,
{
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(cmp::Ordering::Equal)
    }
}

impl<P> Eq for PathConditionFact<P> where P: Hash + Eq + Clone {}

impl<P> PartialOrd for PathConditionFact<P>
where
    P: Hash + Eq + Clone,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        debug_assert_eq!(self.budget, other.budget);
        self.inner.cover.partial_cmp(&other.inner.cover)
    }
}

impl<P> JoinSemiLattice for PathConditionFact<P>
where
    P: Hash + Eq + Clone,
{
    fn join_assign(&mut self, other: Self) -> bool {
        debug_assert_eq!(self.budget, other.budget);
        if other <= *self {
            return false;
        }
        let inner = std::mem::replace(&mut self.inner, PathCondition::zero());
        *self = Self::new(inner | other.inner, self.budget);
        true
    }
}
