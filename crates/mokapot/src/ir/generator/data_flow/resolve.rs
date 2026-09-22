//! Resolution of frame-carrying arms to SSA successor arguments and instruction origins.

use std::collections::HashMap;

use super::{
    Position,
    analysis::{BlockSolution, FrameArm, FrameBlock, FrameSource},
};
use crate::ir::{BasicBlock, BlockId, BlockParameter, SourceMap, Successor, ValueId};

impl BlockSolution {
    fn entry_arguments(&self) -> Vec<ValueId> {
        let frame = self
            .incoming_frames
            .get(&FrameSource::Entry)
            .expect("the entry block is seeded with its entry frame");
        self.parameters
            .keys()
            .map(|&position| {
                frame
                    .value_at(position)
                    .copied()
                    .expect("every incoming frame holds each declared parameter")
            })
            .collect()
    }

    fn parameter_positions(&self) -> Vec<Position> {
        self.parameters.keys().copied().collect()
    }

    fn resolve(self, target_parameters: &HashMap<BlockId, Vec<Position>>) -> BasicBlock {
        let Self {
            parameters, block, ..
        } = self;
        let FrameBlock {
            kind,
            operations,
            terminator,
            ..
        } = block;
        let terminator = terminator.map_arms(|arm| match arm {
            FrameArm::Block {
                target,
                transfer,
                frame,
                ..
            } => {
                let arguments = target_parameters
                    .get(&target)
                    .expect("every arm target is a solved block")
                    .iter()
                    .map(|&position| {
                        frame
                            .value_at(position)
                            .copied()
                            .expect("a successor frame holds every target parameter")
                    })
                    .collect();
                Successor::Block {
                    target,
                    arguments,
                    transfer,
                }
            }
            FrameArm::Unwind { .. } => Successor::Unwind,
        });
        let parameters = parameters
            .into_values()
            .map(|value| BlockParameter { value })
            .collect();
        BasicBlock {
            kind,
            parameters,
            operations: operations
                .into_iter()
                .map(|(_, operation)| operation)
                .collect(),
            terminator,
        }
    }
}

/// Resolves block parameters and successor arguments after fixed-point analysis.
pub(super) fn resolve_blocks(
    entry: BlockId,
    blocks: HashMap<BlockId, BlockSolution>,
) -> (Vec<ValueId>, HashMap<BlockId, BasicBlock>) {
    let entry_arguments = blocks
        .get(&entry)
        .expect("the entry block is always solved")
        .entry_arguments();
    let target_parameters = blocks
        .iter()
        .map(|(&id, block)| (id, block.parameter_positions()))
        .collect::<HashMap<_, _>>();
    let basic_blocks = blocks
        .into_iter()
        .map(|(id, block)| (id, block.resolve(&target_parameters)))
        .collect();
    (entry_arguments, basic_blocks)
}

impl SourceMap {
    pub(super) fn from_block_solutions(block_solutions: &HashMap<BlockId, BlockSolution>) -> Self {
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
