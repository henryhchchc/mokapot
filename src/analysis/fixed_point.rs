//! Module for fixed point analysis
use std::collections::{BTreeMap, VecDeque};

/// A trait for fixed-point analysis.

pub trait FixedPointAnalyzer {
    /// The type of the location in the control flow graph.
    type Location;
    /// The type of the fact that is propagated through the control flow graph.
    type Fact;
    /// The type of the error that can occur during the analysis.
    type Err;

    /// Creates the fact at the entry point of the method being analyzed.
    /// # Errors
    /// - [`Err`] If the fail to create the entry fact.
    fn entry_fact(&self) -> Result<(Self::Location, Self::Fact), Self::Err>;

    /// Executes the method at the given location with the given fact, and returns a map of the affected locations and the corresponding facts.
    /// # Errors
    /// - [`Err`] If the analysis fails.
    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<BTreeMap<Self::Location, Self::Fact>, Self::Err>;

    /// Merges two facts where the control flow joins.
    /// # Errors
    /// - [`Err`] If an error occurred during merging two facts
    fn merge_facts(
        &self,
        current_fact: &Self::Fact,
        incoming_fact: Self::Fact,
    ) -> Result<Self::Fact, Self::Err>;
}

/// Runs fixed-point analysis on a given analyzer, and returns a map of the facts (at fixed points) for each location in the control flow graph.
/// # Errors
/// - [`A::Err`](FixedPointAnalyzer::Err) If the analysis fails.
pub fn analyze<A>(analyzer: &mut A) -> Result<BTreeMap<A::Location, A::Fact>, A::Err>
where
    A: FixedPointAnalyzer,
    <A as FixedPointAnalyzer>::Location: Ord + Eq,
    <A as FixedPointAnalyzer>::Fact: PartialEq,
{
    let mut facts: BTreeMap<A::Location, A::Fact> = BTreeMap::new();
    let entry_node = analyzer.entry_fact()?;
    let mut dirty_nodes = VecDeque::from([entry_node]);

    while let Some((location, incoming_fact)) = dirty_nodes.pop_front() {
        let maybe_updated_fact = match facts.get(&location) {
            Some(current_fact) => {
                let merged_fact = analyzer.merge_facts(current_fact, incoming_fact)?;
                Some(merged_fact).filter(|it| it.ne(current_fact))
            }
            None => Some(incoming_fact),
        };

        if let Some(fact) = maybe_updated_fact {
            dirty_nodes.extend(analyzer.analyze_location(&location, &fact)?);
            facts.insert(location, fact);
        }
    }

    Ok(facts)
}
