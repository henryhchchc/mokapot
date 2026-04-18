use std::convert::Infallible;

use super::{PathCondition, Value};
use crate::{
    analysis::fixed_point::DataflowProblem,
    ir::{ControlFlowGraph, control_flow::ControlTransfer, expression::Condition},
    jvm::code::ProgramCounter,
};

/// An analyzer for path conditions.
#[derive(Debug)]
pub struct Analyzer<'a, N> {
    cfg: &'a ControlFlowGraph<N, ControlTransfer>,
}

impl<'a, N> Analyzer<'a, N> {
    /// Creates a new path condition analyzer.
    #[must_use]
    pub const fn new(cfg: &'a ControlFlowGraph<N, ControlTransfer>) -> Self {
        Self { cfg }
    }
}

impl<'cfg, N> DataflowProblem for Analyzer<'cfg, N> {
    type Location = ProgramCounter;

    type Fact = PathCondition<&'cfg Condition<Value>>;

    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(self.cfg.entry_point(), PathCondition::one())]
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
                    ControlTransfer::Conditional(condition) => fact.clone() & condition.as_ref(),
                    _ => fact.clone(),
                };
                (!propagated.is_contradiction()).then_some((edge.target, propagated))
            })
            .collect::<Vec<_>>())
    }
}
