use std::convert::Infallible;

use super::{PathConditionBudget, Value, cover::PathConditionFact};
use crate::{
    analysis::fixed_point::DataflowProblem,
    ir::{ControlFlowGraph, control_flow::ControlTransfer, expression::Condition},
    jvm::code::ProgramCounter,
};

/// A forward dataflow analysis that propagates path conditions through a CFG.
#[derive(Debug)]
pub(super) struct Analyzer<'a, N> {
    cfg: &'a ControlFlowGraph<N, ControlTransfer>,
    budget: PathConditionBudget,
}

impl<'a, N> Analyzer<'a, N> {
    /// Creates a path-condition analysis over `cfg`.
    #[must_use]
    pub(super) const fn new(
        cfg: &'a ControlFlowGraph<N, ControlTransfer>,
        budget: PathConditionBudget,
    ) -> Self {
        Self { cfg, budget }
    }
}

impl<'cfg, N> DataflowProblem for Analyzer<'cfg, N> {
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
                let propagated = match edge.data {
                    ControlTransfer::Conditional(condition) => {
                        fact.conjoin_branch_guard(condition.as_ref())
                    }
                    _ => fact.clone(),
                };
                (!propagated.is_contradiction()).then_some((edge.target, propagated))
            })
            .collect::<Vec<_>>())
    }
}
