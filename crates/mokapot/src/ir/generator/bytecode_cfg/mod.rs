//! A structural control-flow graph over decoded JVM bytecode.
//!
//! This phase deliberately precedes frame analysis. It validates every decoded
//! instruction, including bytecode unreachable from method entry, before
//! partitioning supported bytecode into reachable blocks.

mod builder;
mod control_flow;

use std::collections::HashMap;

use super::error::Error;
use crate::{
    ir::BlockId,
    jvm::{
        Method,
        code::{Instruction, MethodBody, ProgramCounter},
    },
};

pub(super) use control_flow::{ArmId, ControlFlow, Handler, Target};

/// Builds the reachable control-flow graph used by the later block analyzer.
pub(super) fn build(method: &Method) -> Result<Cfg<'_>, Error> {
    builder::Builder::for_method(method)?.build()
}

/// A control-flow graph whose block identities are fixed before frame
/// propagation begins.
pub(crate) struct Cfg<'method> {
    /// The validated method represented by this graph.
    method: &'method Method,
    /// The block containing the first decoded instruction.
    entry: BlockId,
    /// Exactly the reachable blocks, keyed by identity.
    pub blocks: HashMap<BlockId, Block>,
}

impl<'method> Cfg<'method> {
    /// The validated method represented by this graph.
    pub const fn method(&self) -> &'method Method {
        self.method
    }

    /// The validated method body represented by this graph.
    pub const fn body(&self) -> &'method MethodBody {
        self.method
            .body
            .as_ref()
            .expect("a validated CFG method must have a body")
    }

    /// The block containing the first decoded instruction.
    pub const fn entry_block(&self) -> BlockId {
        self.entry
    }

    /// Looks up a block by its identity.
    pub fn block(&self, id: BlockId) -> &Block {
        self.blocks
            .get(&id)
            .expect("a CFG block identity must belong to its graph")
    }

    /// Iterates over the decoded instructions from `start` through `end`.
    pub fn instructions_in(
        &self,
        start: ProgramCounter,
        end: ProgramCounter,
    ) -> impl DoubleEndedIterator<Item = (ProgramCounter, &Instruction)> {
        self.body().instructions.range(start..=end)
    }
}

/// One block of the control-flow graph.
pub(crate) enum Block {
    /// A decoded bytecode block whose final instruction carries `control`.
    Bytecode {
        /// The first decoded instruction in the block.
        start_pc: ProgramCounter,
        /// The final decoded instruction in the block.
        end_pc: ProgramCounter,
        /// The control transfer of the final instruction.
        control: ControlFlow<BlockId>,
    },
    /// A synthetic landing pad: exactly one unconditional edge into the
    /// handler's bytecode block. It owns no instruction and no location.
    HandlerEntry {
        /// The handler's bytecode block.
        successor: BlockId,
    },
}

impl Block {
    /// The control transfer of a bytecode block, or `None` for a handler entry.
    pub(crate) const fn control(&self) -> Option<&ControlFlow<BlockId>> {
        match self {
            Self::Bytecode { control, .. } => Some(control),
            Self::HandlerEntry { .. } => None,
        }
    }
}
