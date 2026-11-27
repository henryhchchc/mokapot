//! Propagates JVM frames through reachable blocks, constructing provisional SSA.

mod analysis;
mod frame;
mod lifting;
mod resolve;
mod values;

#[cfg(test)]
mod tests;
use std::collections::HashMap;

use super::error::Error;
use crate::ir::{BasicBlock, BlockId, MethodEntry, SourceMap, ValueId};
pub(super) use frame::FrameError;
use frame::{Frame, Position, StackOperation};

pub(super) fn analyze<'class>(
    cfg: &super::control_flow::Cfg<'class>,
) -> Result<IrParts<'class>, Error> {
    analysis::DataflowSolver::new(cfg)?
        .solve()
        .map(resolve::resolve)
}

pub(super) struct IrParts<'class> {
    pub entry: MethodEntry,
    pub blocks: HashMap<BlockId, BasicBlock<'class>>,
    pub this: Option<ValueId>,
    pub parameters: Vec<ValueId>,
    pub source_map: SourceMap,
}
