//! Propagates JVM frames through reachable blocks, constructing provisional SSA.

mod analysis;
mod frame;
pub(super) mod lifting;
mod resolve;
mod values;

use std::collections::HashMap;

pub use frame::FrameError;
use frame::{Frame, Position, StackOperation};

use crate::ir::{
    BlockId, SourceMap,
    generator::{dataflow::analysis::BlockSolution, draft::DraftMethod, error::Error},
};

pub(super) fn analyze(cfg: &super::cfg::Cfg<'_>) -> Result<(DraftMethod, SourceMap), Error> {
    let analysis::DataflowParts {
        entry,
        blocks: block_solutions,
        this_value,
        parameter_values,
    } = analysis::DataflowSolver::new(cfg)?.solve()?;

    let source_map = SourceMap::from_block_solutions(&block_solutions);
    let (entry_arguments, blocks) = resolve::resolve_blocks(entry, block_solutions);

    let draft_method = DraftMethod {
        entry,
        entry_arguments,
        blocks,
        this_value,
        parameter_values,
    };
    Ok((draft_method, source_map))
}

impl SourceMap {
    fn from_block_solutions(block_solutions: &HashMap<BlockId, BlockSolution>) -> Self {
        let mut source_map = SourceMap::new();
        for (&block, sol) in block_solutions {
            if let Some(origin) = sol.block.terminator_source {
                source_map.record_terminator(origin, block);
            }
            for (index, (origin, _)) in sol.block.operations.iter().enumerate() {
                source_map.record_operation(*origin, block, index);
            }
        }
        source_map
    }
}
