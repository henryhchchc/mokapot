//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`bytecode_cfg`] partitions all decoded bytecode into a structural CFG.
//! 2. [`bytecode_analysis`] analyzes reachable structural blocks, lifts their
//!    instructions, and constructs provisional SSA with explicit
//!    predecessor-indexed phis.
//! 3. [`canonicalize`] simplifies provisional phis and materializes retained
//!    phis in canonical SSA blocks.
//! 4. [`finish`] indexes stable value definitions and assembles the completed
//!    [`MokaIRMethod`].
//!
//! The [`bytecode_analysis::lifting`] module contains the JVM opcode semantics
//! used while analyzing structural blocks.

mod bytecode_analysis;
mod bytecode_cfg;
mod canonicalize;
mod error;
mod finish;
mod remap;

pub use bytecode_analysis::FrameError as MokaIRFrameError;
pub use error::{Error as MokaIRBuildError, MalformedBytecode, UnsupportedBytecode};

use crate::{ir::MokaIRMethod, jvm::Method};

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let cfg = bytecode_cfg::build(method)?;
    let (scalar_graph, source_map) = bytecode_analysis::analyze(&cfg)?;
    let canonical_graph = canonicalize::canonicalize(scalar_graph)?;
    let ir = finish::finish(method, canonical_graph, source_map)?;
    #[cfg(test)]
    crate::ir::verify::verify(&ir)
        .unwrap_or_else(|error| panic!("generated invalid Moka IR: {error}"));
    Ok(ir)
}

#[cfg(test)]
mod tests;
