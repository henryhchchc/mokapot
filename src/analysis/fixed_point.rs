//! Module for implementing fixed point analysis algorithms.
//!
//! Fixed point analysis is a technique used in static program analysis where a set of facts
//! (or dataflow information) is propagated through a control flow graph until a stable state
//! (fixed point) is reached. This module provides a generic framework for implementing
//! various fixed point analyses.
//!
//! # Example
//!
//! ```ignore
//! use mokapot::analysis::fixed_point::Analyzer;
//! use std::collections::BTreeMap;
//!
//! // Implement the Analyzer trait for your specific analysis
//! struct MyAnalyzer { /* ... */ }
//!
//! impl Analyzer for MyAnalyzer {
//!     // Define your types and implement required methods
//!     // ...
//! }
//!
//! // Run the analysis
//! let mut analyzer = MyAnalyzer { /* ... */ };
//! let result: BTreeMap<Location, Fact> = analyzer.analyze().expect("Analysis failed");
//! ```
use std::collections::{BTreeMap, BTreeSet};

/// A trait for implementing fixed-point analysis algorithms.
///
/// This trait provides the building blocks needed to implement a fixed-point analysis
/// by defining the data types and operations necessary for propagating facts through
/// a control flow graph until a stable state is reached.
///
/// Implementors must define how facts are created at entry points, how they're transformed
/// at each location, and how they're merged when control flow paths join.
pub trait Analyzer {
    /// The type representing a location in the control flow graph.
    ///
    /// This could be a basic block, an instruction, or any other unit of the program
    /// at which analysis facts need to be computed.
    type Location;

    /// The type of fact that is propagated through the control flow graph.
    ///
    /// Facts represent the information being tracked by the analysis, such as
    /// available expressions, live variables, or type information.
    type Fact;

    /// The type of error that can occur during the analysis.
    ///
    /// This allows the analysis to report specific errors when computation fails.
    type Err;

    /// The type representing a collection of locations and their propagated facts.
    ///
    /// This represents facts that are propagated to various locations during analysis.
    /// It must be an iterable collection of (Location, Fact) pairs, where each pair
    /// indicates a location and the fact that should be propagated to it.
    type PropagatedFacts: IntoIterator<Item = (Self::Location, Self::Fact)>;

    /// Creates the initial facts at the entry point(s) of the analysis.
    ///
    /// This method establishes the starting facts for the analysis. It should return
    /// an iterable of (location, fact) pairs representing the entry points of the
    /// control flow graph and their associated initial facts.
    ///
    /// # Errors
    ///
    /// Returns `Err` if creating the entry facts fails for any reason.
    fn entry_fact(&self) -> Result<Self::PropagatedFacts, Self::Err>;

    /// Analyzes a specific location with a given fact.
    ///
    /// This method implements the transfer function of the analysis. Given a location
    /// and a fact, it computes the resulting facts at all successor locations that are
    /// directly affected by this computation. The method returns an iterable collection
    /// of (location, fact) pairs representing all the affected locations and their
    /// associated facts.
    ///
    /// # Parameters
    ///
    /// * `location` - The current location being analyzed
    /// * `fact` - The current fact at this location
    ///
    /// # Errors
    ///
    /// Returns `Err` if the analysis fails at this location.
    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::PropagatedFacts, Self::Err>;

    /// Merges facts where control flow paths converge.
    ///
    /// When multiple control flow paths join, this method combines the facts from
    /// these paths. The implementation of this method determines how information
    /// is preserved or approximated when paths merge. For example, in a may-analysis
    /// it might compute a union of sets, while in a must-analysis it might compute
    /// an intersection.
    ///
    /// # Parameters
    ///
    /// * `current_fact` - The existing fact at a location
    /// * `incoming_fact` - A new fact arriving at the same location
    ///
    /// # Returns
    ///
    /// The merged fact that incorporates information from both input facts.
    ///
    /// # Errors
    ///
    /// Returns `Err` if merging the facts fails.
    fn merge_facts(
        &self,
        current_fact: &Self::Fact,
        incoming_fact: Self::Fact,
    ) -> Result<Self::Fact, Self::Err>;

    /// Runs the fixed-point analysis algorithm.
    ///
    /// This method iteratively propagates facts through the control flow graph until a fixed point
    /// is reached (i.e., until no facts change during an iteration). It returns a map containing
    /// the final stabilized facts for each analyzed location.
    ///
    /// The implementation uses a worklist algorithm:
    /// 1. Initialize facts at entry points
    /// 2. Process locations from the worklist until it's empty
    /// 3. For each location, compute new facts and propagate them to affected locations
    /// 4. If a fact at a location changes, add all affected locations to the worklist
    /// 5. Continue until the worklist is empty (fixed point reached)
    ///
    /// # Type Constraints
    ///
    /// * `Self::Location` must implement `Ord` and `Eq` for deterministic ordering in the `BTreeMap`
    /// * `Self::Fact` must implement `Ord` and `Eq` for deterministic ordering and equality comparisons
    ///
    /// # Returns
    ///
    /// A `BTreeMap` mapping each location to its final fact at the fixed point.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the analysis fails at any point during fact propagation or merging.
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

        while let Some((location, incoming_facts)) = dirty_nodes.pop_first() {
            let incoming_fact = merge_incoming_facts(self, incoming_facts)?;
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

/// Merges multiple incoming facts into a single fact.
///
/// This helper function takes a set of facts and merges them into a single fact by
/// repeatedly applying the analyzer's `merge_facts` method. This is used when multiple
/// facts need to be combined at a single location.
///
/// # Type Parameters
///
/// * `A` - The type implementing the `Analyzer` trait
///
/// # Parameters
///
/// * `analyzer` - The analyzer containing the merge logic
/// * `incoming_facts` - A set of facts to be merged
///
/// # Returns
///
/// The single merged fact that combines all incoming facts.
///
/// # Errors
///
/// Returns `Err` if merging any of the facts fails.
fn merge_incoming_facts<A: Analyzer + ?Sized>(
    analyzer: &mut A,
    incoming_facts: BTreeSet<A::Fact>,
) -> Result<A::Fact, A::Err> {
    // TODO: Replace it with `try_reduce` when it's stable.
    //       See https://github.com/rust-lang/rust/issues/87053.
    let mut merged_fact = None;
    for incoming_fact in incoming_facts {
        if let Some(ref merged) = merged_fact {
            let new = analyzer.merge_facts(merged, incoming_fact)?;
            merged_fact.replace(new);
        } else {
            merged_fact.replace(incoming_fact);
        }
    }
    Ok(merged_fact.unwrap())
}
