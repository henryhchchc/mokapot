//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`instruction_graph`] produces a reachable JVM instruction graph with
//!    exact register instructions and edge frames.
//! 2. [`block_formation`] consumes that graph, groups its locations and edge
//!    frames into maximal JVM blocks, and classifies their scalar operations and
//!    explicit terminators.
//! 3. [`ssa`] collects and simplifies predecessor-indexed phis, then materializes
//!    scalar operands directly into semantic blocks.
//! 4. [`emission`] assigns public identities and emits the completed [`MokaIRMethod`].
//!
//! The [`instruction_graph::lifting`] module contains the JVM opcode semantics
//! used while constructing the instruction graph.

mod block_formation;
mod emission;
mod error;
mod identity;
mod instruction_graph;
mod ssa;

pub use error::Error as MokaIRBuildError;

use crate::{ir::MokaIRMethod, jvm::Method};

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let instruction_graph = instruction_graph::build(method)?;
    let block_graph = block_formation::form(instruction_graph)?;
    let ssa_graph = ssa::construct(block_graph)?;
    emission::emit(method, ssa_graph)
}

#[cfg(test)]
mod tests;
