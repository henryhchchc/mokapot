//! A structural control-flow graph over decoded JVM bytecode.
//!
//! This phase deliberately precedes frame analysis. It validates every decoded
//! instruction, including bytecode unreachable from method entry, before
//! partitioning supported bytecode into blocks.

mod builder;
mod fallibility;

use std::collections::BTreeMap;

use derive_more::From;

use super::error::Error;
use crate::jvm::{Method, code::ProgramCounter, references::ClassRef};

/// Builds the decoded-bytecode CFG used by the later block analyzer.
pub(super) fn build(method: &Method) -> Result<JvmBlockGraph, Error> {
    builder::Builder::for_method(method)?.build()
}

/// A block-first CFG that preserves decoded JVM bytecode structure.
#[derive(Debug, Clone)]
pub(super) struct JvmBlockGraph {
    /// The block containing the first decoded instruction.
    entry: JvmBlockId,
    /// The blocks, keyed by identity.
    blocks: BTreeMap<JvmBlockId, JvmBlock>,
}

impl JvmBlockGraph {
    /// The block containing the first decoded instruction.
    pub const fn entry_block(&self) -> JvmBlockId {
        self.entry
    }

    /// Looks up a bytecode block by its identity.
    pub fn block(&self, id: JvmBlockId) -> Option<&JvmBlock> {
        self.blocks.get(&id)
    }
}

/// A bytecode block ending at an ordinary transfer or fallible instruction.
#[derive(Debug, Clone)]
pub(super) struct JvmBlock {
    /// The first decoded instruction in the block.
    pub start_pc: ProgramCounter,
    /// The final decoded instruction in the block.
    pub end_pc: ProgramCounter,
    /// The ordinary control-flow topology after the final instruction.
    pub exit: BlockExit,
    /// Ordered exceptional successors of the final fallible instruction.
    pub exception_handlers: Vec<ExceptionalTarget>,
}

/// The identity of a structural bytecode block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub(super) struct JvmBlockId(#[from] ProgramCounter);

/// The target of an exceptional structural control-flow edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ExceptionalTarget {
    /// A synthetic entry that installs the caught exception before entering the
    /// handler `block`.
    Handler {
        /// The bytecode block the selected handler enters.
        block: JvmBlockId,
        /// The exception type selected by this arm, or `None` for catch-all.
        catch_type: Option<ClassRef>,
    },
    /// The synthetic exit for an exception that escapes the method.
    Unwind,
}

/// The control-flow topology at the end of a structural block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BlockExit {
    /// Ordinary execution continues at the following block.
    Fallthrough { target: JvmBlockId },
    /// An unconditional static jump.
    Goto { target: JvmBlockId },
    /// A conditional static jump and its required fallthrough.
    Branch {
        taken: JvmBlockId,
        fallthrough: JvmBlockId,
    },
    /// A static switch dispatch. Each key is retained, including coincident targets.
    Switch {
        cases: BTreeMap<i32, JvmBlockId>,
        default: JvmBlockId,
    },
    /// A return or explicit throw with no ordinary successor.
    Terminal,
}
