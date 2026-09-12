//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`jvm::analysis`] produces a reachable JVM control-flow graph with
//!    exact symbolic instructions and edge frames.
//! 2. [`block_formation`] consumes that graph, groups its locations and edge
//!    frames into maximal JVM blocks, and classifies their scalar operations and
//!    explicit terminators.
//! 3. [`ssa`] collects and simplifies predecessor-indexed phis, then materializes
//!    scalar operands directly into semantic blocks.
//! 4. [`emission`] assigns public identities and emits the completed [`MokaIRMethod`].
//!
//! The [`jvm::lifting`] module contains the JVM opcode semantics used by frame
//! analysis.

mod block_formation;
mod emission;
mod error;
mod identity;
mod jvm;
mod ssa;

pub use error::MokaIRBuildError;
#[allow(
    unused_imports,
    reason = "the re-export preserves the generator error surface"
)]
pub use jvm::frame::ExecutionError;

use crate::{ir::MokaIRMethod, jvm::Method};
use jvm::analysis::JvmFrameAnalyzer;

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let analyzed_cfg = JvmFrameAnalyzer::for_method(method)?.run()?;
    let block_graph = block_formation::form(analyzed_cfg)?;
    let ssa_graph = ssa::construct(block_graph)?;
    emission::emit(method, ssa_graph)
}

#[cfg(test)]
mod tests;
