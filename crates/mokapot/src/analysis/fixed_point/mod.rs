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
//! ```ignore
//! use mokapot::analysis::fixed_point::{DataflowProblem, JoinSemiLattice, solve};
//! use std::collections::BTreeMap;
//!
//! #[derive(Clone, PartialEq, PartialOrd)]
//! struct MyFact { /* ... */ }
//!
//! impl JoinSemiLattice for MyFact {
//!     fn join_assign(&mut self, other: Self) -> bool { /* ... */ }
//! }
//!
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
//! // The inferred map type selects the container: `BTreeMap` here, `HashMap`
//! // when the location is not `Ord`.
//! let mut analysis = MyAnalysis { /* ... */ };
//! let results: BTreeMap<_, _> = solve(&mut analysis).expect("analysis failed");
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
