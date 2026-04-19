use std::{convert::Infallible, hash::Hash};

use crate::{
    analysis::fixed_point::{DataflowProblem, JoinSemiLattice},
    ir::{
        ControlFlowGraph,
        control_flow::{ControlTransfer, PathCondition, PathConditionBudget, Value},
        expression::Condition,
    },
    jvm::code::ProgramCounter,
};

use super::BranchGuard;

/// A forward dataflow analysis that propagates path conditions through a CFG.
#[derive(Debug)]
pub(super) struct PathConditionProblem<'a, N> {
    cfg: &'a ControlFlowGraph<N, ControlTransfer>,
    budget: PathConditionBudget,
}

impl<'a, N> PathConditionProblem<'a, N> {
    /// Creates a path-condition analysis over `cfg`.
    #[must_use]
    pub(super) const fn new(
        cfg: &'a ControlFlowGraph<N, ControlTransfer>,
        budget: PathConditionBudget,
    ) -> Self {
        Self { cfg, budget }
    }
}

impl<'cfg, N> DataflowProblem for PathConditionProblem<'cfg, N> {
    type Location = ProgramCounter;

    type Fact = PathConditionFact<&'cfg Condition<Value>>;

    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(self.cfg.entry_point(), PathConditionFact::one(self.budget))]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        Ok(self
            .cfg
            .outgoing_edges(*location)
            .into_iter()
            .flatten()
            .filter_map(|edge| {
                let propagated = if let ControlTransfer::Conditional(condition) = edge.data {
                    fact.conjoin_branch_guard(condition.as_ref())
                } else {
                    fact.clone()
                };
                (!propagated.is_contradiction()).then_some((edge.target, propagated))
            })
            .collect::<Vec<_>>())
    }
}

/// Internal lattice wrapper used by the generic fixed-point solver.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub(super) struct PathConditionFact<P> {
    inner: PathCondition<P>,
    budget: PathConditionBudget,
}

impl<P> PathConditionFact<P> {
    pub(crate) fn one(budget: PathConditionBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::new(PathCondition::one(), budget)
    }

    pub(crate) fn new(inner: PathCondition<P>, budget: PathConditionBudget) -> Self
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
    fn join(self, other: Self) -> Self {
        debug_assert_eq!(self.budget, other.budget);
        Self::new(self.inner | other.inner, self.budget)
    }
}

#[cfg(test)]
mod tests {
    use super::{PathConditionBudget, PathConditionFact};
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

        let fact = PathConditionFact::new(structural, PathConditionBudget::default());

        assert_eq!(fact.into_inner(), PathCondition::of(a));
    }

    #[test]
    fn fact_join_reduces_after_structural_union() {
        let a = BooleanVariable::Positive(1_u32);
        let b = BooleanVariable::Positive(2_u32);
        let lhs = PathConditionFact::new(
            PathCondition::of(a.clone()) & b.clone(),
            PathConditionBudget::default(),
        );
        let rhs = PathConditionFact::new(
            PathCondition::of(a.clone()) & !b,
            PathConditionBudget::default(),
        );

        assert_eq!(lhs.join(rhs).into_inner(), PathCondition::of(a));
    }
}
