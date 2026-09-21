//! A structural control-flow graph over decoded JVM bytecode.
//!
//! This phase deliberately precedes dataflow analysis. It validates every decoded
//! instruction, including bytecode unreachable from method entry, before
//! partitioning supported bytecode into reachable blocks.

mod builder;
mod control_flow;
mod fallibility;
mod layout;

use std::collections::HashMap;

pub(super) use control_flow::{ArmKey, BlockExit, ExceptionArm, ExceptionTarget};

use super::error::Error;
use crate::{
    ir::BlockId,
    jvm::{
        Method,
        code::{Instruction, MethodBody, ProgramCounter},
    },
};

/// Builds the reachable control-flow graph used by the later dataflow stage.
pub(super) fn build(method: &Method) -> Result<Cfg<'_>, Error> {
    builder::build(method)
}

/// A control-flow graph whose block identities are fixed before dataflow
/// propagation begins.
pub(crate) struct Cfg<'method> {
    method: &'method Method,
    body: &'method MethodBody,
    entry: BlockId,
    blocks: HashMap<BlockId, CfgNode>,
}

impl<'method> Cfg<'method> {
    pub const fn method(&self) -> &'method Method {
        self.method
    }

    pub const fn body(&self) -> &'method MethodBody {
        self.body
    }

    pub const fn entry_block(&self) -> BlockId {
        self.entry
    }

    pub fn block(&self, id: BlockId) -> &CfgNode {
        self.blocks
            .get(&id)
            .expect("a CFG block identity must belong to its graph")
    }

    pub fn instructions_in(
        &self,
        start: ProgramCounter,
        end: ProgramCounter,
    ) -> impl DoubleEndedIterator<Item = (ProgramCounter, &Instruction)> {
        self.body().instructions.range(start..=end)
    }
}

pub(crate) enum CfgNode {
    Code {
        start_pc: ProgramCounter,
        end_pc: ProgramCounter,
        exit: BlockExit<BlockId>,
    },
    LandingPad {
        successor: BlockId,
    },
}
