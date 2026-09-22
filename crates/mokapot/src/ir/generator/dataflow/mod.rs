//! Propagates JVM frames through reachable blocks, constructing provisional SSA.

mod analysis;
mod frame;
mod lifting;
mod resolve;
mod values;

#[cfg(test)]
mod tests;
use std::collections::HashMap;

pub use frame::FrameError;
use frame::{Frame, Position, StackOperation};
#[cfg(test)]
pub(super) use tests::verify_method;

use crate::ir::{BasicBlock, BlockId, MethodEntry, SourceMap, ValueId, generator::error::Error};

pub(super) fn analyze(cfg: &super::cfg::Cfg<'_>) -> Result<IrParts, Error> {
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
            target: entry,
            arguments: entry_arguments,
        },
        blocks,
        this: this_value,
        parameters: parameter_values,
        source_map,
    };
    Ok(parts_method)
}

pub(super) struct IrParts {
    pub entry: MethodEntry,
    pub blocks: HashMap<BlockId, BasicBlock>,
    pub this: Option<ValueId>,
    pub parameters: Vec<ValueId>,
    pub source_map: SourceMap,
}
