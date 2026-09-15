//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`jvm::symbolic_execution`] produces a reachable JVM control-flow graph with
//!    exact register instructions and edge frames.
//! 2. [`block_formation`] consumes that graph, groups its locations and edge
//!    frames into maximal JVM blocks, and classifies their scalar operations and
//!    explicit terminators.
//! 3. [`ssa`] collects and simplifies predecessor-indexed phis, then materializes
//!    scalar operands directly into semantic blocks.
//! 4. [`emission`] assigns public identities and emits the completed [`MokaIRMethod`].
//!
//! The [`jvm::symbolic_execution::lifting`] module contains the JVM opcode
//! semantics used by symbolic execution.

mod block_formation;
mod emission;
mod error;
mod identity;
mod jvm;
mod ssa;

pub use error::Error as MokaIRBuildError;

use crate::{ir::MokaIRMethod, jvm::Method};

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let symbolic_cfg = jvm::build_symbolic_cfg(method)?;
    let block_graph = block_formation::form(symbolic_cfg)?;
    let ssa_graph = ssa::construct(block_graph)?;
    emission::emit(method, ssa_graph)
}

#[cfg(test)]
mod tests;
