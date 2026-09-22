//! Module for implementing fixed-point dataflow analysis algorithms.
//!
//! This module provides a generic framework for implementing iterative dataflow analyses
//! using standard abstractions from program analysis theory:
//!
//! - [`JoinSemiLattice`]: Defines the algebraic structure for dataflow facts
//! - [`DataflowProblem`]: Defines the analysis problem (initial facts + flow function)
//! - [`FactsMap`]: Abstraction over map data structures (e.g., `BTreeMap`, `HashMap`)
//! - [`solve`]: Runs the worklist algorithm
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
//!     fn join_assign(&mut self, other: Self) -> bool { /* ... */ }
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
    collections::{BTreeMap, HashMap},
    hash::{BuildHasher, Hash},
};

/// A join semi-lattice for dataflow analysis.
///
/// Facts form a partially ordered set in which every pair has a least upper
/// bound, the join (⊔). The join defines how facts combine where control flow
/// merges, and [`join_assign`](Self::join_assign) computes it in place.
///
/// # Laws
///
/// The join must be idempotent (`a ⊔ a = a`), commutative (`a ⊔ b = b ⊔ a`),
/// and associative (`(a ⊔ b) ⊔ c = a ⊔ (b ⊔ c)`).
///
/// # Ordering
///
/// [`PartialOrd`] expresses the lattice ordering (⊑): `a <= b` means `a` is no
/// more informative than `b`. It must agree with the join, making `a ⊔ b` the
/// *least* upper bound of `a` and `b`. This ordering may differ from any
/// "natural" ordering of the type; a powerset lattice, for example, has
/// `{a} <= {a, b}`.
///
/// # Termination
///
/// The fixed-point algorithm terminates when the lattice has finite height
/// (all ascending chains are finite) and the flow function is monotonic.
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait JoinSemiLattice: PartialOrd {
    /// Joins `other` into `self` in place.
    ///
    /// Implementations should reuse owned storage from either operand where
    /// practical. Returns `true` exactly when `self` changed, which must move
    /// `self` strictly upwards in the lattice ordering.
    fn join_assign(&mut self, other: Self) -> bool;
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
/// Neither the `Location` nor the `Fact` type requires [`Clone`]: the `Fact`
/// only requires [`JoinSemiLattice`] (which includes `PartialOrd`), and
/// locations move between the two maps rather than being copied. The choice of
/// container (e.g., `BTreeMap` vs `HashMap`) is made at the call site of
/// [`solve`], allowing flexibility based on what traits your types implement.
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
    type Location;

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
    /// Solvers accumulate propagated facts and cannot retract an earlier
    /// contribution. Consequently, successors may appear as the input grows,
    /// but an existing successor or its propagated information must not disappear.
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

/// A map of dataflow facts that also serves as the solver's worklist.
///
/// A location's fact is owned by exactly one map at a time: [`pop_one`] hands a
/// pending `(location, fact)` to the result map, and successor facts are joined
/// back into the worklist. Locations are therefore moved, not cloned.
///
/// # Provided Implementations
///
/// - [`BTreeMap<L, F>`] for `L: Ord`
/// - [`HashMap<L, F>`] for `L: Hash + Eq`
///
/// [`pop_one`]: FactsMap::pop_one
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait FactsMap<L, F>: Default {
    /// Joins `fact` into the fact stored at `location`.
    ///
    /// If no fact exists at the location, the new fact is inserted directly.
    /// Otherwise the new fact is joined in place, so this does not require
    /// [`Clone`].
    ///
    /// # Returns
    ///
    /// The stored location and joined fact when the stored fact changed, or
    /// `None` when the new fact added no information.
    fn insert_or_join(&mut self, location: L, fact: F) -> Option<(&L, &F)>
    where
        F: JoinSemiLattice;

    /// Removes and returns an arbitrary `(location, fact)` entry, or `None`
    /// when empty.
    fn pop_one(&mut self) -> Option<(L, F)>;
}

impl<L, F> FactsMap<L, F> for BTreeMap<L, F>
where
    L: Ord,
{
    fn insert_or_join(&mut self, location: L, fact: F) -> Option<(&L, &F)>
    where
        F: JoinSemiLattice,
    {
        use std::collections::btree_map::Entry;
        let entry = match self.entry(location) {
            Entry::Vacant(entry) => entry.insert_entry(fact),
            Entry::Occupied(mut entry) => {
                if !entry.get_mut().join_assign(fact) {
                    return None;
                }
                entry
            }
        };
        // SAFETY: `entry` borrows `self`, so its key and value outlive the
        // returned references; the local binding only obscures that lifetime.
        Some(unsafe {
            (
                std::mem::transmute::<&L, &L>(entry.key()),
                std::mem::transmute::<&F, &F>(entry.get()),
            )
        })
    }

    fn pop_one(&mut self) -> Option<(L, F)> {
        self.pop_first()
    }
}

impl<L, F, S> FactsMap<L, F> for HashMap<L, F, S>
where
    L: Hash + Eq,
    S: BuildHasher + Default,
{
    fn insert_or_join(&mut self, location: L, fact: F) -> Option<(&L, &F)>
    where
        F: JoinSemiLattice,
    {
        use std::collections::hash_map::Entry;
        let entry = match self.entry(location) {
            Entry::Vacant(entry) => entry.insert_entry(fact),
            Entry::Occupied(mut entry) => {
                if !entry.get_mut().join_assign(fact) {
                    return None;
                }
                entry
            }
        };
        // SAFETY: `entry` borrows `self`, so its key and value outlive the
        // returned references; the local binding only obscures that lifetime.
        Some(unsafe {
            (
                std::mem::transmute::<&L, &L>(entry.key()),
                std::mem::transmute::<&F, &F>(entry.get()),
            )
        })
    }

    fn pop_one(&mut self) -> Option<(L, F)> {
        let location = self.keys().next()?;
        // SAFETY: `remove_entry` searches with `location` before it moves the
        // matching entry and never reads the key afterwards, so disassociating
        // its lifetime from the map is sound for this call.
        let location = unsafe { std::mem::transmute::<&L, &L>(location) };
        self.remove_entry(location)
    }
}

/// Computes the fixed point of a dataflow analysis problem.
///
/// This function implements a worklist algorithm that iteratively propagates
/// dataflow facts through the control flow graph until no more changes occur.
///
/// # Algorithm
///
/// 1. Join seed facts into the worklist
/// 2. While the worklist is non-empty:
///    a. Remove a pending `(location, fact)` entry
///    b. Join its fact into the result map
///    c. If the stored fact changed, apply the flow function and join each
///       successor fact back into the worklist
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
    M: FactsMap<P::Location, P::Fact>,
{
    let mut facts = M::default();
    let mut worklist = M::default();

    for (location, fact) in problem.seeds() {
        worklist.insert_or_join(location, fact);
    }

    while let Some((location, incoming)) = worklist.pop_one() {
        let Some((location, fact)) = facts.insert_or_join(location, incoming) else {
            continue;
        };
        for (successor, propagated) in problem.flow(location, fact)? {
            worklist.insert_or_join(successor, propagated);
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
/// - `Some(x) ⊔ Some(y) = Some(x ⊔ y)` (lifted join)
/// - `None ⊔ Some(x) = Some(x)` (bottom identity)
/// - `None <= Some(_)` for all values
/// - `Some(a) <= Some(b)` iff `a <= b` in the inner lattice
impl<T: JoinSemiLattice> JoinSemiLattice for Option<T> {
    fn join_assign(&mut self, other: Self) -> bool {
        match (self, other) {
            (_, None) => false,
            (slot @ None, Some(other)) => {
                *slot = Some(other);
                true
            }
            (Some(this), Some(other)) => this.join_assign(other),
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::{BTreeMap, BTreeSet},
        convert::Infallible,
    };

    use proptest::prelude::*;

    use crate::analysis::fixed_point::{DataflowProblem, JoinSemiLattice, solve};

    #[derive(Debug, Clone, PartialEq, Eq, proptest_derive::Arbitrary)]
    struct TestSet(BTreeSet<u8>);

    impl PartialOrd for TestSet {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            if self == other {
                Some(std::cmp::Ordering::Equal)
            } else if self.0.is_subset(&other.0) {
                Some(std::cmp::Ordering::Less)
            } else if self.0.is_superset(&other.0) {
                Some(std::cmp::Ordering::Greater)
            } else {
                None
            }
        }
    }

    impl JoinSemiLattice for TestSet {
        fn join_assign(&mut self, other: Self) -> bool {
            let old_len = self.0.len();
            self.0.extend(other.0);
            self.0.len() != old_len
        }
    }

    struct RepeatedSuccessors {
        one_calls: usize,
    }

    impl DataflowProblem for RepeatedSuccessors {
        type Location = u8;
        type Fact = TestSet;
        type Err = Infallible;

        fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
            [(0, TestSet(BTreeSet::new()))]
        }

        fn flow(
            &mut self,
            location: &Self::Location,
            _fact: &Self::Fact,
        ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
            Ok(match location {
                0 => vec![
                    (1, TestSet(BTreeSet::from([1]))),
                    (1, TestSet(BTreeSet::from([2]))),
                ],
                1 => {
                    self.one_calls += 1;
                    Vec::new()
                }
                _ => unreachable!("the test problem only names locations 0 and 1"),
            })
        }
    }

    #[test]
    fn worklist_coalesces_repeated_successors() {
        let mut problem = RepeatedSuccessors { one_calls: 0 };

        let facts: BTreeMap<_, _> = solve(&mut problem).expect("infallible analysis");

        assert_eq!(facts[&1], TestSet(BTreeSet::from([1, 2])));
        assert_eq!(problem.one_calls, 1);
    }

    proptest! {
       #[test]
       fn option_join_ordering(
           lhs in any::<Option<TestSet>>(),
           rhs in any::<Option<TestSet>>(),
       ) {
           let mut joined = lhs.clone();
           let changed = joined.join_assign(rhs.clone());
           prop_assert!(joined >= lhs);
           prop_assert!(joined >= rhs);
           prop_assert_eq!(changed, joined != lhs);

           // The join is commutative and idempotent.
           let mut commuted = rhs.clone();
           commuted.join_assign(lhs.clone());
           prop_assert_eq!(&joined, &commuted);
           prop_assert!(!joined.clone().join_assign(joined.clone()));
       }
    }
}
