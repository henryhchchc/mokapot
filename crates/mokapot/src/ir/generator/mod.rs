//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`cfg`] partitions all decoded bytecode into a structural CFG.
//! 2. [`dataflow`] analyzes reachable structural blocks and
//!    constructs a mutable parts IR with provisional SSA.
//! 3. [`canonicalize`] simplifies provisional block parameters.
//! 4. [`definitions`] indexes each SSA value to its defining site.
//!
//! Analysis fixes addressable instruction positions and records their JVM
//! origins in a detached source map. Later phases preserve those positions.
//!
//! The [`dataflow::lifting`] module contains the JVM opcode semantics
//! used while analyzing structural blocks.

mod canonicalize;
mod cfg;
mod dataflow;
mod definitions;
mod error;
mod remap;

pub use dataflow::FrameError as MokaIRFrameError;
pub use error::{Error as MokaIRBuildError, MalformedBytecode, UnsupportedBytecode};

use crate::{ir::MokaIRMethod, jvm::Method};

pub(super) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let cfg = cfg::build(method)?;
    let dataflow::IrParts {
        mut entry,
        mut blocks,
        this,
        parameters,
        source_map,
    } = dataflow::analyze(&cfg)?;
    canonicalize::canonicalize_values(&mut entry, &mut blocks);
    let ir = MokaIRMethod {
        access_flags: method.access_flags,
        name: method.name.clone(),
        descriptor: method.descriptor.clone(),
        owner: method.owner.clone(),
        source_map,
        entry,
        value_definitions: definitions::index_definitions(this, &parameters, &blocks),
        this,
        parameters,
        blocks,
    };
    Ok(ir)
}

#[cfg(test)]
mod tests;
