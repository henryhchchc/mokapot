//! Propagates JVM frames through reachable blocks, constructing provisional SSA.

mod analysis;
mod frame;
mod lifting;
mod resolve;
mod values;

#[cfg(test)]
mod tests;
pub use frame::FrameError;
use frame::{Frame, Position, StackOperation};
#[cfg(test)]
pub(super) use tests::verify_method;

use crate::ir::{
    SourceMap,
    generator::{draft::DraftMethod, error::Error},
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
        this: this_value,
        parameters: parameter_values,
    };
    Ok((draft_method, source_map))
}
