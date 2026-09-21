use std::{cmp, collections::HashMap, convert::Infallible, hash::Hash};

use super::{BranchGuard, PathCondition, SolvingBudget};
use crate::{
    analysis::fixed_point::{DataflowProblem, JoinSemiLattice},
    ir::{
        BasicBlock, BlockId,
        control_flow::{ControlTransfer, outgoing_edges},
        expression::Predicate,
    },
};

/// A forward dataflow analysis that propagates path conditions through a CFG.
#[derive(Debug)]
pub(super) struct PathConditionProblem<'method> {
    blocks: &'method HashMap<BlockId, BasicBlock>,
    entry: BlockId,
    budget: SolvingBudget,
}

impl<'method> PathConditionProblem<'method> {
    /// Creates a path-condition analysis over the given method blocks.
    #[must_use]
    pub(super) const fn new(
        blocks: &'method HashMap<BlockId, BasicBlock>,
        entry: BlockId,
        budget: SolvingBudget,
    ) -> Self {
        Self {
            blocks,
            entry,
            budget,
        }
    }
}

impl<'method> DataflowProblem for PathConditionProblem<'method> {
    type Location = BlockId;

    type Fact = PathConditionFact<&'method Predicate>;

    type Err = Infallible;

    type Output = Vec<(Self::Location, Self::Fact)>;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(self.entry, PathConditionFact::one(self.budget))]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::Output, Self::Err> {
        Ok(outgoing_edges(self.blocks, *location)
            .filter_map(|edge| {
                let propagated = match edge.transfer() {
                    ControlTransfer::Conditional(guard) => {
                        fact.conjoin_branch_guard(guard.as_ref())
                    }
                    ControlTransfer::Unconditional | ControlTransfer::Exception(_) => fact.clone(),
                };
                (!propagated.is_contradiction()).then_some((edge.target(), propagated))
            })
            .collect::<Vec<_>>())
    }
}

/// Internal lattice wrapper used by the generic fixed-point solver.
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
        let inner = inner.reduce_with_budget(budget);
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
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<P> Eq for PathConditionFact<P> where P: Hash + Eq {}

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
