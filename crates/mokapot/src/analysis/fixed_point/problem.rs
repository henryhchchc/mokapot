use super::JoinSemiLattice;

/// A dataflow analysis problem.
///
/// Defines the fact lattice, the initial facts ([`seeds`](Self::seeds)), and the
/// transfer function ([`flow`](Self::flow)), which must be monotonic for
/// [`solve`](super::solve) to terminate.
///
/// Neither [`Location`](Self::Location) nor [`Fact`](Self::Fact) requires
/// [`Clone`]: locations move between the solver's maps, and facts join in place.
///
/// [`flow`](Self::flow) takes `&mut self`, so an analysis may accumulate state
/// (e.g., build an IR) across the traversal.
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait DataflowProblem {
    /// A program point where facts are computed, such as a basic block.
    type Location;

    /// The fact computed at each location.
    type Fact: JoinSemiLattice;

    /// The error type for fallible operations.
    type Err;

    /// Returns the initial `(location, fact)` pairs.
    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)>;

    /// Propagates `fact` from `location` to its successors.
    ///
    /// Must be monotonic: `fact₁ ⊑ fact₂` implies `flow(location, fact₁) ⊑
    /// flow(location, fact₂)` component-wise. The solver accumulates
    /// contributions and never retracts them, so successors and their propagated
    /// information must not disappear as the input grows.
    ///
    /// # Errors
    ///
    /// Returns an error if the transfer cannot be computed.
    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err>;
}
