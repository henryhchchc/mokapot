//! A structural control-flow graph over decoded JVM bytecode.
//!
//! This phase deliberately precedes frame analysis. It validates every decoded
//! instruction, including bytecode unreachable from method entry, before
//! partitioning supported bytecode into blocks.

mod builder;
mod fallibility;
mod instruction_flow;
mod model;

pub(super) use model::{
    Block, BranchPredicate, BytecodeCfg, ExceptionalTarget, HandlerId, ReturnOperand,
    StructuralBlockId, StructuralTerminator,
};

use super::error::Error;
use crate::jvm::Method;

/// Builds the decoded-bytecode CFG used by the later block analyzer.
pub(super) fn build(method: &Method) -> Result<BytecodeCfg, Error> {
    builder::Builder::for_method(method)?.build()
}

#[cfg(test)]
mod tests;
