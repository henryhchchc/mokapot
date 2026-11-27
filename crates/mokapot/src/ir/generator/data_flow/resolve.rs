//! Resolution of frame-carrying arms to SSA successor arguments and instruction origins.

use std::collections::HashMap;

use super::{
    Frame, IrParts, Position,
    analysis::{BlockSolution, DataflowParts, FrameArm, FrameBlock, FrameSource},
};
use crate::ir::{BasicBlock, BlockId, BlockParameter, MethodEntry, SourceMap, Successor, ValueId};

impl<'class> BlockSolution<'class> {
    fn entry_arguments(&self) -> Vec<ValueId> {
        let frame = self
            .incoming_frames
            .get(&FrameSource::Entry)
            .expect("the entry block is seeded with its entry frame");
        arguments_for(frame, self.parameters.keys().copied())
    }

    fn resolve(
        self,
        id: BlockId,
        target_parameters: &HashMap<BlockId, Vec<Position>>,
        source_map: &mut SourceMap,
    ) -> BasicBlock<'class> {
        let Self {
            parameters, block, ..
        } = self;
        let FrameBlock {
            kind,
            operations,
            terminator,
            terminator_source,
        } = block;
        let terminator = terminator.map_arms(|arm| match arm {
            FrameArm::Block {
                target,
                transfer,
                frame,
                ..
            } => {
                let positions = target_parameters
                    .get(&target)
                    .expect("every arm target is a solved block");
                Successor::Block {
                    target,
                    arguments: arguments_for(&frame, positions.iter().copied()),
                    transfer,
                }
            }
            FrameArm::Unwind { .. } => Successor::Unwind,
        });
        let parameters = parameters
            .into_values()
            .map(|value| BlockParameter { value })
            .collect();
        if let Some(origin) = terminator_source {
            source_map.record_terminator(origin, id);
        }
        let operations = operations
            .into_iter()
            .enumerate()
            .map(|(index, (origin, operation))| {
                source_map.record_operation(origin, id, index);
                operation
            })
            .collect();
        BasicBlock {
            kind,
            parameters,
            operations,
            terminator,
        }
    }
}

fn arguments_for(frame: &Frame, positions: impl Iterator<Item = Position>) -> Vec<ValueId> {
    positions
        .map(|position| {
            *frame
                .value_at(position)
                .expect("every incoming frame holds each declared parameter")
        })
        .collect()
}

/// Resolves block parameters and successor arguments after fixed-point analysis.
pub(super) fn resolve(parts: DataflowParts<'_>) -> IrParts<'_> {
    let DataflowParts {
        entry,
        blocks,
        this_value,
        parameter_values,
    } = parts;
    let entry_arguments = blocks
        .get(&entry)
        .expect("the entry block is always solved")
        .entry_arguments();
    let target_parameters = blocks
        .iter()
        .map(|(&id, block)| (id, block.parameters.keys().copied().collect()))
        .collect::<HashMap<_, _>>();
    let mut source_map = SourceMap::new();
    let blocks = blocks
        .into_iter()
        .map(|(id, block)| (id, block.resolve(id, &target_parameters, &mut source_map)))
        .collect();
    IrParts {
        entry: MethodEntry {
            block: entry,
            arguments: entry_arguments,
        },
        blocks,
        this: this_value,
        parameters: parameter_values,
        source_map,
    }
}
