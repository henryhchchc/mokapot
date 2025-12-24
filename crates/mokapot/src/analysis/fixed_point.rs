//! Module for implementing fixed-point dataflow analysis algorithms.
//!
//! This module provides a generic framework for implementing iterative dataflow analyses
//! using standard abstractions from program analysis theory:
//!
//! - [`JoinSemiLattice`]: Defines the algebraic structure for dataflow facts
//! - [`DataflowProblem`]: Defines the analysis problem (initial facts + flow function)
//! - [`FactsMap`]: Abstraction over map data structures (e.g., `BTreeMap`, `HashMap`)
//! - [`solve`]: Runs the worklist algorithm to compute the fixed point
//!
//! # Theoretical Background
//!
//! Fixed-point analysis iteratively propagates dataflow facts through a control flow graph
//! until reaching a stable state where no more changes occur. The framework is based on:
//!
//! - **Join semi-lattice**: Facts form a partially ordered set with a join (⊔) operation
//!   that computes the least upper bound of two elements.
//! - **Flow functions**: Transform facts at each program location to produce facts for
//!   successor locations.
//! - **Monotonicity**: Flow functions must be monotonic to guarantee termination.
//!
//! # Example
//!
//! ```ignore
//! use mokapot::analysis::fixed_point::{DataflowProblem, JoinSemiLattice, solve};
//! use std::collections::BTreeMap;
//!
//! // Define your fact type with lattice operations
//! #[derive(Clone, PartialEq, PartialOrd)]
//! struct MyFact { /* ... */ }
//!
//! impl JoinSemiLattice for MyFact {
//!     fn join(&self, other: &Self) -> Self { /* ... */ }
//! }
//!
//! // Define your analysis problem
//! struct MyAnalysis { /* ... */ }
//!
//! impl DataflowProblem for MyAnalysis {
//!     type Location = usize;
//!     type Fact = MyFact;
//!     type Err = std::convert::Infallible;
//!
//!     fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> { /* ... */ }
//!     fn flow(&mut self, loc: &Self::Location, fact: &Self::Fact)
//!         -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> { /* ... */ }
//! }
//!
//! // Run the analysis with BTreeMap as the container (requires Ord)
//! let mut analysis = MyAnalysis { /* ... */ };
//! let results: BTreeMap<_, _> = solve(&mut analysis).expect("Analysis failed");
//!
//! // Or use HashMap for non-Ord types
//! // let results: HashMap<_, _> = solve(&mut analysis).expect("Analysis failed");
//! ```

use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    hash::{BuildHasher, Hash},
};

/// A join semi-lattice for dataflow analysis.
///
/// A join semi-lattice is a partially ordered set where every pair of elements has a
/// least upper bound (join). This algebraic structure is fundamental to dataflow analysis
/// as it defines how facts are combined when control flow paths merge.
///
/// # Laws
///
/// Implementations must satisfy the following laws:
///
/// - **Idempotency**: `a.clone().join(a) == a`
/// - **Commutativity**: `a.join(b) == b.join(a)`
/// - **Associativity**: `a.join(b).join(c) == a.join(b.join(c))`
///
/// # Lattice Ordering via `PartialOrd`
///
/// This trait requires [`PartialOrd`] to express the lattice ordering (⊑). The ordering
/// represents information content: `a <= b` means "a is less informative than or equal to b".
///
/// The lattice ordering must be consistent with the join operation:
/// - `a <= a.join(b)` and `b <= a.join(b)` (join is an upper bound)
/// - If `a <= c` and `b <= c`, then `a.join(b) <= c` (join is the *least* upper bound)
///
/// **Note**: This ordering may differ from any "natural" ordering of the underlying type.
/// For example, in a powerset lattice, `{a} <= {a, b}` even though set ordering might
/// typically be defined differently.
///
/// # Termination
///
/// For the fixed-point algorithm to terminate, the lattice should have finite height
/// (i.e., all ascending chains are finite), or the analysis should use widening.
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait JoinSemiLattice: PartialOrd {
    /// Computes the join (least upper bound) of two elements.
    ///
    /// The join operation combines information from two facts, typically when
    /// control flow paths merge. For may-analyses, this is usually set union;
    /// for must-analyses, set intersection.
    ///
    /// This method consumes both operands, similar to [`std::ops::Add`]. This
    /// allows implementations to reuse allocations when possible. If you need
    /// to keep the original values, clone them before calling `join`.
    ///
    /// # Arguments
    ///
    /// * `other` - The other element to join with
    ///
    /// # Returns
    ///
    /// The least upper bound of `self` and `other`.
    #[must_use]
    fn join(self, other: Self) -> Self;
}

/// A dataflow analysis problem definition.
///
/// This trait encapsulates everything needed to define a dataflow analysis:
/// - The types of locations and facts
/// - The initial facts (seeds) at entry points
/// - The flow function that transforms facts at each location
///
/// The flow function should be monotonic with respect to the lattice ordering
/// to guarantee termination of the fixed-point algorithm.
///
/// # Type Parameters
///
/// Implementors define three associated types:
/// - [`Location`](DataflowProblem::Location): Program points in the control flow graph
/// - [`Fact`](DataflowProblem::Fact): Dataflow facts that form a join semi-lattice
/// - [`Err`](DataflowProblem::Err): Error type for fallible operations
///
/// # Container Independence
///
/// The `Location` type only requires `Clone`. The `Fact` type only requires
/// [`JoinSemiLattice`] (which includes `PartialOrd`).
/// The choice of container (e.g., `BTreeMap` vs `HashMap`) is made at the call
/// site of [`solve`], allowing flexibility based on what traits your types implement.
///
/// # Mutability
///
/// The [`flow`](Self::flow) method takes `&mut self` to support analyses that need
/// to accumulate state during traversal (e.g., building an IR, collecting statistics).
/// For pure analyses that don't require mutation, simply don't mutate `self` in the
/// implementation—Rust's borrow checker handles this correctly.
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait DataflowProblem {
    /// The type representing a location in the control flow graph.
    ///
    /// This could be a basic block, an instruction, or any program point
    /// where dataflow facts are computed.
    type Location: Clone;

    /// The type of dataflow fact being computed.
    ///
    /// Must implement [`JoinSemiLattice`] to define how facts are combined
    /// at control flow merge points. The solver uses the lattice ordering
    /// (via `PartialOrd`) to detect when facts have stabilized.
    type Fact: JoinSemiLattice;

    /// The error type for operations that may fail.
    type Err;

    /// Returns the initial facts (seeds) for the analysis.
    ///
    /// Seeds are the starting points for the analysis, typically the entry
    /// point(s) of the control flow graph with their initial facts. The
    /// worklist algorithm begins by processing these seeds.
    ///
    /// In IFDS terminology, these are the "seed" facts from which the analysis
    /// propagates.
    ///
    /// # Returns
    ///
    /// An iterator of (location, fact) pairs representing initial facts.
    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)>;

    /// Applies the flow function at a location.
    ///
    /// Given a location and an incoming fact, computes the facts that should
    /// be propagated to successor locations. This is the transfer function
    /// of the analysis.
    ///
    /// The flow function should be **monotonic**: if `fact₁ ⊑ fact₂`, then
    /// `flow(loc, fact₁)` ⊑ `flow(loc, fact₂)` (component-wise on successors).
    ///
    /// # Arguments
    ///
    /// * `location` - The current program location
    /// * `fact` - The incoming dataflow fact at this location
    ///
    /// # Returns
    ///
    /// An iterator of `(successor_location, propagated_fact)` pairs.
    ///
    /// # Errors
    ///
    /// Returns an error if the flow function cannot be computed.
    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err>;
}

/// A trait for map-like containers used in the fixed-point algorithm.
///
/// This abstraction allows the solver to work with different map implementations
/// (e.g., `BTreeMap`, `HashMap`) depending on what traits the key type implements.
///
/// # Provided Implementations
///
/// - [`BTreeMap<L, F>`] for `L: Ord`
/// - [`HashMap<L, F>`] for `L: Hash + Eq`
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait FactsMap<L, F>: Default {
    /// Returns a reference to the fact at the given location, if present.
    fn get(&self, location: &L) -> Option<&F>;

    /// Inserts a fact at the given location, returning the previous fact if any.
    fn insert(&mut self, location: L, fact: F) -> Option<F>;

    /// Inserts a fact, joining with any existing fact at that location.
    ///
    /// If no fact exists at the location, the new fact is inserted directly.
    /// If a fact already exists, the new fact is joined with it.
    fn insert_or_join(&mut self, location: L, fact: F)
    where
        F: JoinSemiLattice;

    /// Removes and returns an arbitrary (location, fact) pair from the map.
    ///
    /// Returns `None` if the map is empty. The order of removal is
    /// implementation-defined.
    fn pop(&mut self) -> Option<(L, F)>;
}

impl<L: Ord, F> FactsMap<L, F> for BTreeMap<L, F> {
    fn get(&self, location: &L) -> Option<&F> {
        BTreeMap::get(self, location)
    }

    fn insert(&mut self, location: L, fact: F) -> Option<F> {
        BTreeMap::insert(self, location, fact)
    }

    fn insert_or_join(&mut self, location: L, fact: F)
    where
        F: JoinSemiLattice,
    {
        match self.remove(&location) {
            Some(existing) => {
                self.insert(location, existing.join(fact));
            }
            None => {
                self.insert(location, fact);
            }
        }
    }

    fn pop(&mut self) -> Option<(L, F)> {
        self.pop_first()
    }
}

impl<L: Hash + Eq + Clone, F, S: BuildHasher + Default> FactsMap<L, F> for HashMap<L, F, S> {
    fn get(&self, location: &L) -> Option<&F> {
        HashMap::get(self, location)
    }

    fn insert(&mut self, location: L, fact: F) -> Option<F> {
        HashMap::insert(self, location, fact)
    }

    fn insert_or_join(&mut self, location: L, fact: F)
    where
        F: JoinSemiLattice,
    {
        match self.remove(&location) {
            Some(existing) => {
                self.insert(location, existing.join(fact));
            }
            None => {
                self.insert(location, fact);
            }
        }
    }

    fn pop(&mut self) -> Option<(L, F)> {
        // HashMap doesn't have pop_first, so we get an arbitrary key and remove it
        let key = self.keys().next().cloned()?;
        let value = self.remove(&key)?;
        Some((key, value))
    }
}

/// Computes the fixed point of a dataflow analysis problem.
///
/// This function implements a worklist algorithm that iteratively propagates
/// dataflow facts through the control flow graph until no more changes occur.
///
/// # Algorithm
///
/// 1. Initialize the worklist with seed facts
/// 2. While the worklist is non-empty:
///    a. Remove a (location, facts) pair from the worklist
///    b. Join all incoming facts at this location
///    c. If the joined fact changes the current fact at this location:
///       - Update the fact at this location
///       - Apply the flow function to compute successor facts
///       - Add successor facts to the worklist
/// 3. Return the final facts at all locations
///
/// # Type Parameters
///
/// * `P` - The dataflow problem to solve
/// * `M` - The map type to use for storing facts (e.g., `BTreeMap`, `HashMap`)
///
/// The map type is inferred from the return type, allowing you to choose the
/// appropriate container based on what traits your `Location` type implements:
///
/// ```ignore
/// // For types implementing Ord:
/// let results: BTreeMap<_, _> = solve(&problem)?;
///
/// // For types implementing Hash + Eq:
/// let results: HashMap<_, _> = solve(&problem)?;
/// ```
///
/// # Errors
///
/// Returns an error if the flow function fails at any location.
///
/// # Termination
///
/// Termination is guaranteed if:
/// - The lattice has finite height (all ascending chains are finite)
/// - The flow function is monotonic
#[instability::unstable(feature = "fixed-point-analyses")]
pub fn solve<P, M>(problem: &mut P) -> Result<M, P::Err>
where
    P: DataflowProblem,
    <P as DataflowProblem>::Fact: Clone,
    M: FactsMap<P::Location, P::Fact>,
{
    let mut facts = M::default();
    let mut worklist = M::default();

    // Initialize worklist with seeds
    for (loc, fact) in problem.seeds() {
        worklist.insert_or_join(loc, fact);
    }

    // Fixed-point iteration
    while let Some((location, incoming_fact)) = worklist.pop() {
        // Compute the new fact by joining with the existing fact (if any)
        let new_fact = match facts.get(&location) {
            Some(current) => current.clone().join(incoming_fact),
            None => incoming_fact,
        };

        // Check if the fact increased in the lattice ordering.
        // In a monotonic analysis, facts only increase, so we check if
        // new_fact is strictly greater than current (i.e., current < new_fact),
        // or if they are incomparable (which shouldn't happen in a well-formed
        // lattice, but we handle it by propagating).
        let increased = facts
            .get(&location)
            .is_none_or(|it| new_fact.partial_cmp(it).is_some_and(Ordering::is_gt));

        if increased {
            // Apply the flow function and propagate to successors
            for (succ_loc, succ_fact) in problem.flow(&location, &new_fact)? {
                worklist.insert_or_join(succ_loc, succ_fact);
            }

            // Update the fact at this location
            facts.insert(location, new_fact);
        }
    }

    Ok(facts)
}

// ============================================================================
// Common Lattice Implementations
// ============================================================================

/// A "lifted" lattice over `Option<T>` where `None` is bottom.
///
/// This constructs a new lattice by adding a bottom element (`None`) below
/// an existing lattice `T`. This is useful when "no information yet" needs
/// to be distinguished from any actual lattice value.
///
/// # Lattice Structure
///
/// - `None` is the bottom element (⊥)
/// - `Some(x).join(Some(y)) = Some(x.join(y))` (lifted join)
/// - `None.join(Some(x)) = Some(x)` (bottom identity)
/// - `None <= Some(_)` for all values
/// - `Some(a) <= Some(b)` iff `a <= b` in the inner lattice
impl<T: JoinSemiLattice> JoinSemiLattice for Option<T> {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (None, None) => None,
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (Some(a), Some(b)) => Some(a.join(b)),
        }
    }
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;

    use crate::{analysis::fixed_point::JoinSemiLattice, ir::Operand};

    proptest! {
       #[test]
       fn operand_join_ordering(
           lhs in any::<Option<Operand>>(),
           rhs in any::<Option<Operand>>(),
       ) {
           let joined = lhs.clone().join(rhs.clone());
           prop_assert!(joined >= lhs);
           prop_assert!(joined >= rhs);
       }
    }
}
