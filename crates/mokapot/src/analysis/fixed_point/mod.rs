//! A generic framework for iterative (fixed-point) dataflow analysis.
//!
//! - [`JoinSemiLattice`]: how facts combine
//! - [`DataflowProblem`]: locations, seeds, and the flow function
//! - [`FactsMap`]: the fact map, doubling as the worklist
//! - [`solve`]: the worklist algorithm
//!
//! Facts form a partially ordered set joined (⊔) where control flow merges. For
//! [`solve`] to terminate, the lattice must have finite height and the flow
//! function must be monotonic.
//!
//! # Example
//!
//! ```
//! # #[cfg(feature = "unstable-fixed-point-analyses")]
//! # {
//! use mokapot::analysis::fixed_point::{DataflowProblem, JoinSemiLattice, solve};
//! use std::{collections::BTreeMap, convert::Infallible};
//!
//! #[derive(Clone, PartialEq, PartialOrd)]
//! struct Reachable(bool);
//!
//! impl JoinSemiLattice for Reachable {
//!     fn join_assign(&mut self, other: Self) -> bool {
//!         let changed = other.0 && !self.0;
//!         self.0 |= other.0;
//!         changed
//!     }
//! }
//!
//! struct Reachability;
//!
//! impl DataflowProblem for Reachability {
//!     type Location = usize;
//!     type Fact = Reachable;
//!     type Err = Infallible;
//!
//!     fn seeds(&self) -> impl IntoIterator<Item = (usize, Reachable)> {
//!         [(0, Reachable(true))]
//!     }
//!
//!     fn flow(&mut self, location: &usize, fact: &Reachable)
//!         -> Result<impl IntoIterator<Item = (usize, Reachable)>, Infallible>
//!     {
//!         // 0 -> 1 -> 2 -> 0; the unchanged fact ends the cycle.
//!         Ok([((location + 1) % 3, fact.clone())])
//!     }
//! }
//!
//! let results: BTreeMap<_, _> = solve(&mut Reachability).unwrap();
//! assert_eq!(results.keys().copied().collect::<Vec<_>>(), [0, 1, 2]);
//! assert!(results.values().all(|fact| fact.0));
//! # }
//! ```

mod facts_map;
mod lattice;
mod problem;
mod solve;

#[cfg(test)]
mod tests;

#[instability::unstable(feature = "fixed-point-analyses")]
pub use facts_map::{FactsMap, QueuedFactsMap};
#[instability::unstable(feature = "fixed-point-analyses")]
pub use lattice::JoinSemiLattice;
#[instability::unstable(feature = "fixed-point-analyses")]
pub use problem::DataflowProblem;
#[instability::unstable(feature = "fixed-point-analyses")]
pub use solve::solve;
