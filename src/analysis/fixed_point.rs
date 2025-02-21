//! Module for fixed point analysis
use std::collections::{BTreeMap, BTreeSet};

/// A trait for fixed-point analysis.
pub trait Analyzer {
    /// The type of the location in the control flow graph.
    type Location;
    /// The type of the fact that is propagated through the control flow graph.
    type Fact;
    /// The type of the error that can occur during the analysis.
    type Err;

    /// The type of the locations that are affected by the analysis.
    type AffectedLocations: IntoIterator<Item = (Self::Location, Self::Fact)>;

    /// Creates the fact at the entry point of the method being analyzed.
    /// # Errors
    /// - [`Err`] If the fail to create the entry fact.
    fn entry_fact(&self) -> Result<Self::AffectedLocations, Self::Err>;

    /// Executes the method at the given location with the given fact, and returns an iterator over
    /// the affected locations and the corresponding facts.
    /// # Errors
    /// - [`Err`] If the analysis fails.
    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::AffectedLocations, Self::Err>;

    /// Merges two facts where the control flow joins.
    /// # Errors
    /// - [`Err`] If an error occurred during merging two facts
    fn merge_facts(
        &self,
        current_fact: &Self::Fact,
        incoming_fact: Self::Fact,
    ) -> Result<Self::Fact, Self::Err>;

    /// Runs fixed-point analysis on a given analyzer, and returns a map of the facts (at fixed points)
    /// for each location in the control flow graph.
    /// # Errors
    /// - [`Analyzer::Err`] If the analysis fails.
    fn analyze(&mut self) -> Result<BTreeMap<Self::Location, Self::Fact>, Self::Err>
    where
        Self::Location: Ord + Eq,
        Self::Fact: Ord + Eq,
    {
        let mut facts: BTreeMap<Self::Location, Self::Fact> = BTreeMap::new();
        let mut dirty_nodes: BTreeMap<_, _> = self
            .entry_fact()?
            .into_iter()
            .map(|(loc, fact)| (loc, BTreeSet::from([fact])))
            .collect();
        //let mut dirty_nodes = BTreeMap::from([(entry_point, BTreeSet::from([entry_fact]))]);

        while let Some((location, incoming_facts)) = dirty_nodes.pop_first() {
            let incoming_fact = {
                // TODO: Replace it with `try_reduce` when it's stable.
                //       See https://github.com/rust-lang/rust/issues/87053.
                let mut merged_fact = None;
                for incoming_fact in incoming_facts {
                    match merged_fact {
                        Some(ref merged) => {
                            let new = self.merge_facts(merged, incoming_fact)?;
                            merged_fact.replace(new);
                        }
                        _ => {
                            merged_fact.replace(incoming_fact);
                        }
                    }
                }
                merged_fact.unwrap()
            };
            let maybe_updated_fact = match facts.get(&location) {
                Some(current_fact) => {
                    let merged_fact = self.merge_facts(current_fact, incoming_fact)?;
                    Some(merged_fact).filter(|it| it != current_fact)
                }
                None => Some(incoming_fact),
            };

            if let Some(fact) = maybe_updated_fact {
                for (loc, new_fact) in self.analyze_location(&location, &fact)? {
                    dirty_nodes.entry(loc).or_default().insert(new_fact);
                }
                facts.insert(location, fact);
            }
        }

        Ok(facts)
    }
}
