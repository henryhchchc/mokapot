//! Solves reachable JVM frames, then emits provisional SSA once.

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
    let analysis::DataflowParts {
        entry,
        blocks,
        this_value,
        parameter_values,
    } = analysis::DataflowSolver::new(cfg)?.solve()?;

    let source_map = SourceMap::from_block_solutions(&blocks);
    let (entry_arguments, blocks) = resolve::resolve_blocks(entry, blocks);

    let parts_method = IrParts {
        entry: MethodEntry {
            block: entry,
            arguments: entry_arguments,
        },
        blocks,
        this: this_value,
        parameters: parameter_values,
        source_map,
    };
    Ok(parts_method)
}

pub(super) struct IrParts<'class> {
    pub entry: MethodEntry,
    pub blocks: HashMap<BlockId, BasicBlock<'class>>,
    pub this: Option<ValueId>,
    pub parameters: Vec<ValueId>,
    pub source_map: SourceMap,
}
