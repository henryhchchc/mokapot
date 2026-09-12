//! Module for implementing fixed-point dataflow analysis algorithms.
//!
//! This module provides a generic framework for implementing iterative dataflow analyses
//! using standard abstractions from program analysis theory:
//!
//! - [`JoinSemiLattice`]: Defines the algebraic structure for dataflow facts
//! - [`DataflowProblem`]: Defines the analysis problem (initial facts + flow function)
//! - [`DataflowOutput`]: Exposes the successor facts produced by a flow function
//! - [`FactsMap`]: Abstraction over map data structures (e.g., `BTreeMap`, `HashMap`)
//! - [`solve`] and an internal recomputing variant: Run the worklist algorithm
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
//!     type Output = Vec<(Self::Location, Self::Fact)>;
//!
//!     fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> { /* ... */ }
//!     fn flow(&mut self, loc: &Self::Location, fact: &Self::Fact)
//!         -> Result<Self::Output, Self::Err> { /* ... */ }
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
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
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
    /// Joins `other` into this element in place.
    ///
    /// Implementations should reuse owned storage from either operand where
    /// practical. The returned boolean must be `true` exactly when the value
    /// of `self` changed. A change must move `self` strictly upwards in the
    /// lattice ordering.
    fn join_assign(&mut self, other: Self) -> bool;

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
    fn join(mut self, other: Self) -> Self
    where
        Self: Sized,
    {
        self.join_assign(other);
        self
    }
}

/// Successor facts produced by a dataflow transfer function.
///
/// The consuming iterator lets [`solve`] propagate facts without cloning them.
/// The borrowed iterator lets the internal recomputing solver retain transfer
/// outputs while comparing and replacing their successor facts.
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait DataflowOutput<L, F> {
    /// Iterates over successor locations and their propagated facts by reference.
    fn successors<'a>(&'a self) -> impl Iterator<Item = (&'a L, &'a F)>
    where
        L: 'a,
        F: 'a;

    /// Consumes this output and iterates over its successor facts.
    fn into_successors(self) -> impl Iterator<Item = (L, F)>;
}

impl<L, F> DataflowOutput<L, F> for Vec<(L, F)> {
    fn successors<'a>(&'a self) -> impl Iterator<Item = (&'a L, &'a F)>
    where
        L: 'a,
        F: 'a,
    {
        self.iter().map(|(location, fact)| (location, fact))
    }

    fn into_successors(self) -> impl Iterator<Item = (L, F)> {
        self.into_iter()
    }
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
/// Implementors define four associated types:
/// - [`Location`](DataflowProblem::Location): Program points in the control flow graph
/// - [`Fact`](DataflowProblem::Fact): Dataflow facts that form a join semi-lattice
/// - [`Err`](DataflowProblem::Err): Error type for fallible operations
/// - [`Output`](DataflowProblem::Output): A transfer result exposing successor facts
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
    type Location;

    /// The type of dataflow fact being computed.
    ///
    /// Must implement [`JoinSemiLattice`] to define how facts are combined
    /// at control flow merge points. The solver uses the lattice ordering
    /// (via `PartialOrd`) to detect when facts have stabilized.
    type Fact: JoinSemiLattice;

    /// The error type for operations that may fail.
    type Err;

    /// The output produced by the transfer function.
    ///
    /// This may be a plain vector of successor facts, or a richer result that
    /// records information such as edge categories or generated instructions.
    type Output: DataflowOutput<Self::Location, Self::Fact>;

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
    /// An output exposing `(successor_location, propagated_fact)` pairs.
    ///
    /// # Errors
    ///
    /// Returns an error if the flow function cannot be computed.
    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::Output, Self::Err>;
}

/// A set-like worklist of dataflow locations.
///
/// Worklists contain only locations. Facts remain in the result map and are
/// therefore not cloned merely to schedule a location for processing.
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait LocationWorklist<L>: Default {
    /// Schedules `location` if it is not already scheduled.
    fn schedule(&mut self, location: L);

    /// Removes and returns an arbitrary scheduled location, or `None` when empty.
    fn pop_one(&mut self) -> Option<L>;
}

impl<L: Ord> LocationWorklist<L> for BTreeSet<L> {
    fn schedule(&mut self, location: L) {
        self.insert(location);
    }

    fn pop_one(&mut self) -> Option<L> {
        self.pop_first()
    }
}

impl<L, S> LocationWorklist<L> for HashSet<L, S>
where
    L: Clone + Hash + Eq,
    S: BuildHasher + Default,
{
    fn schedule(&mut self, location: L) {
        self.insert(location);
    }

    fn pop_one(&mut self) -> Option<L> {
        let location = self.iter().next()?.clone();
        self.take(&location)
    }
}

/// A trait for map-like containers used in the fixed-point algorithm.
///
/// This abstraction allows the solver to work with different map implementations
/// (e.g., `BTreeMap`, `HashMap`) depending on what traits the key type implements.
///
/// # Provided Implementations
///
/// - [`BTreeMap<L, F>`] for `L: Clone + Ord`
/// - [`HashMap<L, F>`] for `L: Clone + Hash + Eq`
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait FactsMap<L, F>: Default {
    /// The location-only worklist compatible with this map's key type.
    type Worklist: LocationWorklist<L>;

    /// Returns the fact stored at `location`, if any.
    fn get(&self, location: &L) -> Option<&F>;

    /// Inserts a fact, joining with any existing fact at that location.
    ///
    /// If no fact exists at the location, the new fact is inserted directly.
    /// If a fact already exists, the new fact is joined with the existing fact.
    ///
    /// # Returns
    ///
    /// Returns `true` if the stored fact changed. Existing facts are updated
    /// in place, so this operation does not require [`Clone`].
    fn insert_or_join(&mut self, location: L, fact: F) -> bool
    where
        F: JoinSemiLattice;
}

impl<L, F> FactsMap<L, F> for BTreeMap<L, F>
where
    L: Clone + Ord,
{
    type Worklist = BTreeSet<L>;

    fn get(&self, location: &L) -> Option<&F> {
        BTreeMap::get(self, location)
    }

    fn insert_or_join(&mut self, location: L, fact: F) -> bool
    where
        F: JoinSemiLattice,
    {
        use std::collections::btree_map::Entry;
        match self.entry(location) {
            Entry::Vacant(entry) => {
                entry.insert(fact);
                true
            }
            Entry::Occupied(mut entry) => entry.get_mut().join_assign(fact),
        }
    }
}

impl<L, F, S> FactsMap<L, F> for HashMap<L, F, S>
where
    L: Clone + Hash + Eq,
    S: BuildHasher + Default,
{
    type Worklist = HashSet<L, S>;

    fn get(&self, location: &L) -> Option<&F> {
        HashMap::get(self, location)
    }

    fn insert_or_join(&mut self, location: L, fact: F) -> bool
    where
        F: JoinSemiLattice,
    {
        use std::collections::hash_map::Entry;
        match self.entry(location) {
            Entry::Vacant(entry) => {
                entry.insert(fact);
                true
            }
            Entry::Occupied(mut entry) => entry.get_mut().join_assign(fact),
        }
    }
}

/// Computes the fixed point of a dataflow analysis problem.
///
/// This function implements a worklist algorithm that iteratively propagates
/// dataflow facts through the control flow graph until no more changes occur.
///
/// # Algorithm
///
/// 1. Join seed facts into the result map and schedule their locations
/// 2. While the worklist is non-empty:
///    a. Remove a location from the worklist
///    b. Apply the flow function to its current joined fact
///    c. Join each successor fact directly into the result map
///    d. Schedule successors whose facts changed
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
/// # Panics
///
/// Panics if a custom [`FactsMap`] or its associated [`LocationWorklist`]
/// violates the storage and scheduling contracts.
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
    P::Location: Clone,
    M: FactsMap<P::Location, P::Fact>,
{
    solve_accumulating(problem, |_, output, facts, worklist| {
        for (successor, propagated) in output.into_successors() {
            schedule_if_changed(facts, worklist, successor, propagated);
        }
    })
}

/// Runs the accumulating worklist algorithm, delegating transfer-output handling.
fn solve_accumulating<P, M, HandleOutput>(
    problem: &mut P,
    mut handle_output: HandleOutput,
) -> Result<M, P::Err>
where
    P: DataflowProblem,
    P::Location: Clone,
    M: FactsMap<P::Location, P::Fact>,
    HandleOutput: FnMut(P::Location, P::Output, &mut M, &mut M::Worklist),
{
    let mut facts = M::default();
    let mut worklist = M::Worklist::default();

    for (location, fact) in problem.seeds() {
        schedule_if_changed(&mut facts, &mut worklist, location, fact);
    }

    while let Some(location) = worklist.pop_one() {
        let fact = facts
            .get(&location)
            .expect("scheduled locations must have a stored fact");
        let output = problem.flow(&location, fact)?;
        handle_output(location, output, &mut facts, &mut worklist);
    }

    Ok(facts)
}

/// Joins a propagated fact and schedules its location when the joined fact changed.
fn schedule_if_changed<L, F, M>(facts: &mut M, worklist: &mut M::Worklist, location: L, fact: F)
where
    L: Clone,
    F: JoinSemiLattice,
    M: FactsMap<L, F>,
{
    if facts.insert_or_join(location.clone(), fact) {
        worklist.schedule(location);
    }
}

/// A completed fixed-point solution with retained transfer outputs.
///
/// The facts are the joined incoming facts at each reachable location. The
/// outputs are the latest transfer result computed from each location's final
/// incoming fact.
#[derive(Debug)]
#[instability::unstable(feature = "fixed-point-analyses")]
pub struct FixedPointResult<Facts, Outputs> {
    facts: Facts,
    outputs: Outputs,
}

#[cfg_attr(
    not(feature = "unstable-fixed-point-analyses"),
    allow(
        dead_code,
        reason = "accessors are public only when the unstable API is enabled"
    )
)]
impl<Facts, Outputs> FixedPointResult<Facts, Outputs> {
    /// Returns the final incoming facts.
    pub const fn facts(&self) -> &Facts {
        &self.facts
    }

    /// Returns the latest transfer outputs.
    pub const fn outputs(&self) -> &Outputs {
        &self.outputs
    }

    /// Separates the solution into its incoming facts and transfer outputs.
    pub fn into_parts(self) -> (Facts, Outputs) {
        (self.facts, self.outputs)
    }
}

type OrderedFixedPointResult<P> = FixedPointResult<
    BTreeMap<<P as DataflowProblem>::Location, <P as DataflowProblem>::Fact>,
    BTreeMap<<P as DataflowProblem>::Location, <P as DataflowProblem>::Output>,
>;

fn recompute_fact<L: Ord, F: Clone + JoinSemiLattice>(
    location: &L,
    seeds: &BTreeMap<L, Vec<F>>,
    incoming_edges: &BTreeMap<L, BTreeMap<L, Vec<F>>>,
) -> Option<F> {
    let mut incoming = seeds
        .get(location)
        .into_iter()
        .flatten()
        .chain(
            incoming_edges
                .get(location)
                .into_iter()
                .flat_map(|incoming| incoming.values())
                .flatten(),
        )
        .cloned();
    incoming.next().map(|mut fact| {
        for contribution in incoming {
            fact.join_assign(contribution);
        }
        fact
    })
}

/// Solves a dataflow problem while replacing superseded edge contributions.
///
/// This solver retains the latest contribution from each source location and
/// recomputes a destination fact when that source output changes. This supports
/// transfer artifacts whose symbolic identities can change as a predecessor fact
/// grows without retaining stale identities in downstream facts.
pub(crate) fn solve_with_recomputed_outputs<P>(
    problem: &mut P,
) -> Result<OrderedFixedPointResult<P>, P::Err>
where
    P: DataflowProblem,
    P::Location: Clone + Ord,
    P::Fact: Clone,
{
    let mut seeds = BTreeMap::<P::Location, Vec<P::Fact>>::new();
    for (location, fact) in problem.seeds() {
        seeds.entry(location).or_default().push(fact);
    }

    let mut facts = BTreeMap::<P::Location, P::Fact>::new();
    let mut outputs = BTreeMap::<P::Location, P::Output>::new();
    let mut incoming_edges = BTreeMap::<P::Location, BTreeMap<P::Location, Vec<P::Fact>>>::new();
    let mut outgoing_targets = BTreeMap::<P::Location, BTreeSet<P::Location>>::new();
    let mut dirty = seeds.keys().cloned().collect::<BTreeSet<_>>();
    let mut worklist = BTreeSet::new();

    while !dirty.is_empty() || !worklist.is_empty() {
        while let Some(location) = dirty.pop_first() {
            match recompute_fact(&location, &seeds, &incoming_edges) {
                Some(fact) if facts.get(&location) != Some(&fact) => {
                    facts.insert(location.clone(), fact);
                    worklist.insert(location);
                }
                None if facts.remove(&location).is_some() => {
                    worklist.remove(&location);
                    outputs.remove(&location);
                    if let Some(targets) = outgoing_targets.remove(&location) {
                        for target in targets {
                            let incoming = incoming_edges
                                .get_mut(&target)
                                .expect("an outgoing target has incoming contributions");
                            incoming.remove(&location);
                            if incoming.is_empty() {
                                incoming_edges.remove(&target);
                            }
                            dirty.insert(target);
                        }
                    }
                }
                Some(_) | None => {}
            }
        }

        let Some(location) = worklist.pop_first() else {
            continue;
        };
        let fact = facts
            .get(&location)
            .expect("scheduled locations must have a stored fact");
        let output = problem.flow(&location, fact)?;
        let mut outgoing = BTreeMap::<P::Location, Vec<P::Fact>>::new();
        for (target, contribution) in output.successors() {
            outgoing
                .entry(target.clone())
                .or_default()
                .push(contribution.clone());
        }
        let targets = outgoing.keys().cloned().collect::<BTreeSet<_>>();
        let previous_targets = outgoing_targets.remove(&location).unwrap_or_default();
        let affected = previous_targets
            .iter()
            .chain(outgoing.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for target in affected {
            let replacement = outgoing.remove(&target);
            if incoming_edges
                .get(&target)
                .and_then(|incoming| incoming.get(&location))
                == replacement.as_ref()
            {
                continue;
            }
            if let Some(replacement) = replacement {
                incoming_edges
                    .entry(target.clone())
                    .or_default()
                    .insert(location.clone(), replacement);
            } else if let Some(incoming) = incoming_edges.get_mut(&target) {
                incoming.remove(&location);
                if incoming.is_empty() {
                    incoming_edges.remove(&target);
                }
            }
            dirty.insert(target);
        }
        if !targets.is_empty() {
            outgoing_targets.insert(location.clone(), targets);
        }
        outputs.insert(location, output);
    }

    Ok(FixedPointResult { facts, outputs })
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
    use std::{collections::BTreeMap, collections::BTreeSet, convert::Infallible};

    use proptest::prelude::*;

    use crate::analysis::fixed_point::{
        DataflowProblem, JoinSemiLattice, solve, solve_with_recomputed_outputs,
    };

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
        type Output = Vec<(Self::Location, Self::Fact)>;

        fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
            [(0, TestSet(BTreeSet::new()))]
        }

        fn flow(
            &mut self,
            location: &Self::Location,
            _fact: &Self::Fact,
        ) -> Result<Self::Output, Self::Err> {
            Ok(match location {
                0 => vec![
                    (1, TestSet(BTreeSet::from([1]))),
                    (1, TestSet(BTreeSet::from([2]))),
                ],
                1 => {
                    self.one_calls += 1;
                    Vec::new()
                }
                _ => unreachable!(),
            })
        }
    }

    #[test]
    fn location_worklist_coalesces_repeated_successors() {
        let mut problem = RepeatedSuccessors { one_calls: 0 };

        let facts: BTreeMap<_, _> = solve(&mut problem).expect("infallible analysis");

        assert_eq!(facts[&1], TestSet(BTreeSet::from([1, 2])));
        assert_eq!(problem.one_calls, 1);
    }

    #[derive(Debug, PartialEq, PartialOrd)]
    struct NonCloneMax(u8);

    impl JoinSemiLattice for NonCloneMax {
        fn join_assign(&mut self, other: Self) -> bool {
            if other > *self {
                *self = other;
                true
            } else {
                false
            }
        }
    }

    struct NonCloneFacts;

    impl DataflowProblem for NonCloneFacts {
        type Location = u8;
        type Fact = NonCloneMax;
        type Err = Infallible;
        type Output = Vec<(Self::Location, Self::Fact)>;

        fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
            [(0, NonCloneMax(1))]
        }

        fn flow(
            &mut self,
            location: &Self::Location,
            _fact: &Self::Fact,
        ) -> Result<Self::Output, Self::Err> {
            Ok(if *location == 0 {
                vec![(1, NonCloneMax(2))]
            } else {
                Vec::new()
            })
        }
    }

    #[test]
    fn ordinary_solver_does_not_require_cloneable_facts() {
        let facts: BTreeMap<_, _> =
            solve(&mut NonCloneFacts).expect("infallible non-clone analysis");

        assert_eq!(facts[&1], NonCloneMax(2));
    }

    struct ReplacingSuccessor;

    impl DataflowProblem for ReplacingSuccessor {
        type Location = u8;
        type Fact = TestSet;
        type Err = Infallible;
        type Output = Vec<(Self::Location, Self::Fact)>;

        fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
            [(0, TestSet(BTreeSet::from([0])))]
        }

        fn flow(
            &mut self,
            location: &Self::Location,
            fact: &Self::Fact,
        ) -> Result<Self::Output, Self::Err> {
            Ok(match location {
                0 => vec![
                    (1, TestSet(BTreeSet::from([0]))),
                    (2, TestSet(BTreeSet::from([u8::from(fact.0.contains(&1))]))),
                ],
                1 => vec![(0, TestSet(BTreeSet::from([1])))],
                2 => Vec::new(),
                _ => unreachable!(),
            })
        }
    }

    #[test]
    fn recomputed_solver_replaces_superseded_edge_contributions() {
        let result =
            solve_with_recomputed_outputs(&mut ReplacingSuccessor).expect("infallible analysis");

        assert_eq!(result.facts()[&0], TestSet(BTreeSet::from([0, 1])));
        assert_eq!(result.facts()[&2], TestSet(BTreeSet::from([1])));
    }

    proptest! {
       #[test]
       fn option_join_ordering(
           lhs in any::<Option<TestSet>>(),
           rhs in any::<Option<TestSet>>(),
       ) {
           let joined = lhs.clone().join(rhs.clone());
           let mut assigned = lhs.clone();
           let changed = assigned.join_assign(rhs.clone());
           prop_assert!(joined >= lhs);
           prop_assert!(joined >= rhs);
           prop_assert_eq!(&assigned, &joined);
           prop_assert_eq!(changed, assigned != lhs);
       }
    }
}
