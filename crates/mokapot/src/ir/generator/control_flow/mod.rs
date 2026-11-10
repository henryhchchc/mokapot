//! A structural control-flow graph over decoded JVM bytecode.
//!
//! Validates every decoded instruction, including bytecode unreachable from
//! method entry, before partitioning supported bytecode into reachable blocks.

mod block_exit;
mod builder;
mod fallibility;
mod layout;

use std::{collections::HashMap, ops::Range};

pub(super) use block_exit::{ArmKey, BlockExit, ExceptionArm, ExceptionTarget};

use super::error::Error;
use crate::{
    ir::BlockId,
    jvm::{
        Method,
        code::{Instruction, MethodBody, ProgramCounter},
    },
};

/// Builds the reachable control-flow graph used by the later dataflow stage.
pub(super) fn analyze(method: Method<'_>) -> Result<Cfg<'_>, Error> {
    builder::build(method)
}

/// A control-flow graph whose block identities are fixed before dataflow
/// propagation begins.
pub(crate) struct Cfg<'method> {
    method: Method<'method>,
    body: MethodBody<'method>,
    entry: BlockId,
    blocks: HashMap<BlockId, CfgNode<'method>>,
}

impl<'method> Cfg<'method> {
    pub const fn method(&self) -> Method<'method> {
        self.method
    }

    pub const fn body(&self) -> MethodBody<'method> {
        self.body
    }

    pub const fn entry_block(&self) -> BlockId {
        self.entry
    }

    /// # Panics
    ///
    /// Panics unless `id` was allocated by this graph.
    pub fn block(&self, id: BlockId) -> &CfgNode<'method> {
        self.blocks
            .get(&id)
            .expect("a CFG block identity belongs to its graph")
    }

    pub fn instructions_for(
        &self,
        block: BlockId,
    ) -> impl DoubleEndedIterator<Item = (ProgramCounter, Instruction<'method>)> {
        let CfgNode::Code {
            instruction_range, ..
        } = self.block(block)
        else {
            panic!("a landing pad has no bytecode instructions");
        };
        self.body
            .instructions()
            .index_range(instruction_range.clone())
    }
}

#[derive(Debug)]
pub(crate) enum CfgNode<'class> {
    Code {
        start_pc: ProgramCounter,
        instruction_range: Range<usize>,
        exit: BlockExit<'class, BlockId>,
    },
    LandingPad {
        successor: BlockId,
    },
}
