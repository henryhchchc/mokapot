//! Fixed-point dataflow analysis of structural blocks.

mod block_state;
mod execution;
mod frame_block;
mod solver;

use std::collections::HashMap;

pub(super) use block_state::BlockSolution;
pub(super) use frame_block::{FrameArm, FrameBlock, FrameSource};
pub(super) use solver::DataflowSolver;

// Re-exported so the submodules below can reach them through a single `super` hop.
use super::{Frame, Position, lifting, values::ValueContext};
use crate::ir::{BlockId, ValueId};

pub(super) struct DataflowParts {
    pub(super) entry: BlockId,
    pub(super) blocks: HashMap<BlockId, BlockSolution>,
    pub(super) this_value: Option<ValueId>,
    pub(super) parameter_values: Vec<ValueId>,
}
