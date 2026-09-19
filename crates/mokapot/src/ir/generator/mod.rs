//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`bytecode_cfg`] partitions all decoded bytecode into a structural CFG.
//! 2. [`bytecode_analysis`] analyzes reachable structural blocks and
//!    materializes a mutable draft IR with provisional SSA.
//! 3. [`canonicalize`] simplifies provisional block parameters in place.
//! 4. [`finish`] constructs public wrappers and derived indexes.
//!
//! The [`bytecode_analysis::lifting`] module contains the JVM opcode semantics
//! used while analyzing structural blocks.

mod bytecode_analysis;
mod bytecode_cfg;
mod canonicalize;
mod draft;
mod error;
mod finish;
mod remap;

pub use bytecode_analysis::FrameError as MokaIRFrameError;
pub use error::{Error as MokaIRBuildError, MalformedBytecode, UnsupportedBytecode};

use crate::{ir::MokaIRMethod, jvm::Method};

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let cfg = bytecode_cfg::build(method)?;
    let mut draft = bytecode_analysis::analyze(&cfg)?;
    canonicalize::canonicalize(&mut draft)?;
    let ir = finish::finish(method, draft)?;
    #[cfg(test)]
    crate::ir::verify::verify(&ir)
        .unwrap_or_else(|error| panic!("generated invalid Moka IR: {error}"));
    Ok(ir)
}

#[cfg(test)]
mod tests;
