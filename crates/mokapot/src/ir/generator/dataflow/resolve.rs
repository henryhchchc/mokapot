//! Resolution of frame-carrying arms to SSA successor arguments.

use std::collections::HashMap;

use super::{
    Position,
    analysis::{BlockSolution, FrameArm, FrameBlock, FrameSource},
};
use crate::ir::{BasicBlock, BlockId, BlockParameter, Successor, ValueId};

impl BlockSolution {
    fn entry_arguments(&self) -> Vec<ValueId> {
        self.arguments_for(FrameSource::Entry)
    }

    fn arguments_for(&self, source: FrameSource) -> Vec<ValueId> {
        let frame = self
            .incoming_frames
            .get(&source)
            .expect("a block solution must retain every incoming frame");
        self.parameters
            .keys()
            .map(|&position| {
                frame
                    .value_at(position)
                    .copied()
                    .expect("a block parameter must have a value in every incoming frame")
            })
            .collect()
    }

    #[cfg(test)]
    fn verify_successors(&self, source: BlockId, blocks: &HashMap<BlockId, BlockSolution>) {
        for (frame_source, target, frame) in self.block.outgoing_frames(source) {
            let target_block = blocks
                .get(&target)
                .expect("every successor must have a block solution");
            let incoming_frame = target_block
                .incoming_frames
                .get(&frame_source)
                .expect("every successor must have a matching incoming frame");
            assert_eq!(
                incoming_frame, frame,
                "a successor frame must match its recorded incoming frame"
            );
        }
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
                    .expect("every successor must have resolved block parameters")
                    .iter()
                    .map(|&position| {
                        frame
                            .value_at(position)
                            .copied()
                            .expect("every successor frame must supply each target parameter")
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
        .expect("the entry block must have a block solution")
        .entry_arguments();
    #[cfg(test)]
    for (&source, block) in &blocks {
        block.verify_successors(source, &blocks);
    }
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
