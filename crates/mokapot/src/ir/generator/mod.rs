//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`cfg`] partitions all decoded bytecode into a structural CFG.
//! 2. [`dataflow`] analyzes reachable structural blocks and
//!    constructs a mutable draft IR with provisional SSA.
//! 3. [`canonicalize`] simplifies provisional block parameters in place.
//! 4. [`finish`] attaches method metadata and derived indexes.
//!
//! Analysis fixes addressable instruction positions and records their JVM
//! origins in a detached source map. Later phases preserve those positions.
//!
//! The [`dataflow::lifting`] module contains the JVM opcode semantics
//! used while analyzing structural blocks.

mod canonicalize;
mod cfg;
mod dataflow;
mod draft;
mod error;
mod finish;
mod remap;

pub use dataflow::FrameError as MokaIRFrameError;
pub use error::{Error as MokaIRBuildError, MalformedBytecode, UnsupportedBytecode};

use crate::{ir::MokaIRMethod, jvm::Method};

pub(super) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let cfg = cfg::build(method)?;
    let (mut draft, source_map) = dataflow::analyze(&cfg)?;
    canonicalize::canonicalize(&mut draft);
    let ir = finish::finish(method, draft, source_map);
    Ok(ir)
}

#[cfg(test)]
mod tests;
