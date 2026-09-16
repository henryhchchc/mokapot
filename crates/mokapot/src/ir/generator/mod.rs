//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`bytecode_cfg`] partitions all decoded bytecode into a structural CFG.
//! 2. [`bytecode_analysis`] analyzes reachable structural blocks, lifts their
//!    instructions, and constructs explicit predecessor-indexed phis.
//! 3. [`ssa`] simplifies and materializes the scalar phis.
//! 4. [`emission`] assigns public identities and emits the completed [`MokaIRMethod`].
//!
//! The [`bytecode_analysis::lifting`] module contains the JVM opcode semantics
//! used while analyzing structural blocks.

mod bytecode_analysis;
mod bytecode_cfg;
mod emission;
mod error;
mod identity;
mod ssa;

pub use bytecode_analysis::jvm::FrameError as MokaIRFrameError;
pub use error::{Error as MokaIRBuildError, MalformedBytecode};

use crate::{ir::MokaIRMethod, jvm::Method};

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let cfg = bytecode_cfg::build(method)?;
    let scalar_graph = bytecode_analysis::analyze(method, &cfg)?;
    let ssa_graph = ssa::construct(scalar_graph)?;
    emission::emit(method, ssa_graph)
}

#[cfg(test)]
mod tests;
