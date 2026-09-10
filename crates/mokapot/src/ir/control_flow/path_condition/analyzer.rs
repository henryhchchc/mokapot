use std::{convert::Infallible, hash::Hash};

use crate::{
    analysis::fixed_point::{DataflowProblem, JoinSemiLattice},
    ir::control_flow::ControlFlowGraph,
    ir::{
        BlockId,
        control_flow::{ControlTransfer, PathCondition, SolvingBudget},
        expression::Predicate,
    },
};

use super::BranchGuard;

/// A forward dataflow analysis that propagates path conditions through a CFG.
#[derive(Debug)]
pub(super) struct PathConditionProblem<'method> {
    cfg: ControlFlowGraph<'method>,
    budget: SolvingBudget,
}

impl<'method> PathConditionProblem<'method> {
    /// Creates a path-condition analysis over `cfg`.
    #[must_use]
    pub(super) const fn new(cfg: ControlFlowGraph<'method>, budget: SolvingBudget) -> Self {
        Self { cfg, budget }
    }
}

impl<'method> DataflowProblem for PathConditionProblem<'method> {
    type Location = BlockId;

    type Fact = PathConditionFact<&'method Predicate>;

    type Err = Infallible;

    type Output = Vec<(Self::Location, Self::Fact)>;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(self.cfg.entry_block(), PathConditionFact::one(self.budget))]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::Output, Self::Err> {
        Ok(self
            .cfg
            .outgoing_edges(*location)
            .filter_map(|edge| {
                let propagated = if let ControlTransfer::Conditional(condition) = edge.transfer() {
                    fact.conjoin_branch_guard(condition.as_ref())
                } else {
                    fact.clone()
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
    pub(crate) fn one(budget: SolvingBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::new(PathCondition::one(), budget)
    }

    pub(crate) fn new(inner: PathCondition<P>, budget: SolvingBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self {
            inner: inner.reduce_with_budget(budget),
            budget,
        }
    }

    pub(crate) fn conjoin_branch_guard(&self, branch_guard: BranchGuard<P>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::new(self.inner.clone() & branch_guard, self.budget)
    }

    pub(crate) fn is_contradiction(&self) -> bool {
        self.inner.is_contradiction()
    }

    pub(crate) fn into_inner(self) -> PathCondition<P> {
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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
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

#[cfg(test)]
mod tests {
    use super::{PathConditionFact, SolvingBudget};
    use crate::{
        analysis::fixed_point::JoinSemiLattice,
        ir::control_flow::path_condition::{BooleanVariable, PathCondition},
    };

    #[test]
    fn fact_construction_reduces_raw_path_conditions() {
        let a = BooleanVariable::Positive(1_u32);
        let b = BooleanVariable::Positive(2_u32);
        let structural =
            (PathCondition::of(a.clone()) & b.clone()) | (PathCondition::of(a.clone()) & !b);

        let fact = PathConditionFact::new(structural, SolvingBudget::default());

        assert_eq!(fact.into_inner(), PathCondition::of(a));
    }

    #[test]
    fn fact_join_reduces_after_structural_union() {
        let a = BooleanVariable::Positive(1_u32);
        let b = BooleanVariable::Positive(2_u32);
        let lhs = PathConditionFact::new(
            PathCondition::of(a.clone()) & b.clone(),
            SolvingBudget::default(),
        );
        let rhs =
            PathConditionFact::new(PathCondition::of(a.clone()) & !b, SolvingBudget::default());

        assert_eq!(lhs.join(rhs).into_inner(), PathCondition::of(a));
    }
}
