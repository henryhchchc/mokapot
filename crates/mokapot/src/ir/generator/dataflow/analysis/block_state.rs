//! Mutable frame merging and execution state for one block.

use std::collections::{BTreeMap, HashMap};

use super::super::{Frame, Position, values::ValueContext};
use super::frame_block::{FrameBlock, FrameSource};
use crate::{
    ir::{ValueId, generator::error::Error},
    jvm::code::ProgramCounter,
};

/// Mutable analysis state for one reachable block.
#[derive(Debug)]
pub(super) struct BlockState {
    incoming_frames: HashMap<FrameSource, Frame>,
    parameters: BTreeMap<Position, ValueId>,
    execution: ExecutionState,
}

impl BlockState {
    pub(super) fn new(source: FrameSource, frame: Frame) -> Self {
        Self {
            incoming_frames: HashMap::from([(source, frame.clone())]),
            parameters: BTreeMap::new(),
            execution: ExecutionState::Ready(frame),
        }
    }

    pub(super) fn add_frame(
        &mut self,
        source: FrameSource,
        frame: Frame,
        block_pc: Option<ProgramCounter>,
        values: &mut ValueContext,
    ) -> Result<bool, Error> {
        self.incoming_frames.insert(source, frame);
        let (input, parameters) = merge_frames(
            self.incoming_frames.values(),
            &self.parameters,
            block_pc,
            values,
        )?;
        self.parameters = parameters;
        Ok(self.execution.update_input(input))
    }

    pub(super) fn begin_execution(&mut self) -> Frame {
        self.execution.begin()
    }

    pub(super) fn complete(&mut self, block: FrameBlock) {
        self.execution.complete(block);
    }

    pub(super) fn into_solution(self) -> BlockSolution {
        let ExecutionState::Complete(block) = self.execution else {
            panic!("the worklist must drain only after every reachable block completes");
        };
        BlockSolution::new(self.incoming_frames, self.parameters, block)
    }
}

/// One block after fixed-point execution has completed.
#[derive(Debug)]
pub(crate) struct BlockSolution {
    pub(crate) incoming_frames: HashMap<FrameSource, Frame>,
    pub(crate) parameters: BTreeMap<Position, ValueId>,
    pub(crate) block: FrameBlock,
}

impl BlockSolution {
    const fn new(
        incoming_frames: HashMap<FrameSource, Frame>,
        parameters: BTreeMap<Position, ValueId>,
        block: FrameBlock,
    ) -> Self {
        Self {
            incoming_frames,
            parameters,
            block,
        }
    }
}

/// Execution lifecycle of a reachable block.
#[derive(Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "each block moves from Ready through Running to Complete"
)]
enum ExecutionState {
    Ready(Frame),
    Running,
    Complete(FrameBlock),
}

impl ExecutionState {
    fn begin(&mut self) -> Frame {
        let Self::Ready(input) = std::mem::replace(self, Self::Running) else {
            panic!("a scheduled block must be ready");
        };
        input
    }

    fn update_input(&mut self, input: Frame) -> bool {
        if let Self::Ready(current) = self
            && current == &input
        {
            return false;
        }
        *self = Self::Ready(input);
        true
    }

    fn complete(&mut self, block: FrameBlock) {
        let Self::Running = self else {
            panic!("only a running block can finish interpretation");
        };
        *self = Self::Complete(block);
    }
}

fn merge_frames<'frames>(
    frames: impl Iterator<Item = &'frames Frame>,
    existing_parameters: &BTreeMap<Position, ValueId>,
    block_pc: Option<ProgramCounter>,
    values: &mut ValueContext,
) -> Result<(Frame, BTreeMap<Position, ValueId>), Error> {
    let mut frames = frames;
    let mut merged = frames
        .next()
        .expect("a block must be created with its first incoming frame")
        .clone();
    let mut active_parameters = BTreeMap::new();

    for incoming in frames {
        merged
            .merge_from_with(incoming.clone(), |position, lhs, rhs| {
                merge_value(
                    position,
                    lhs,
                    rhs,
                    existing_parameters,
                    &mut active_parameters,
                    values,
                );
                Ok::<_, Error>(())
            })
            .map_err(|error| match block_pc {
                Some(pc) => error.at_instruction(pc),
                None => error,
            })?;
    }

    let parameters = active_parameters
        .into_iter()
        .filter(|(position, result)| merged.value_at(*position) == Some(result))
        .collect();

    Ok((merged, parameters))
}

fn merge_value(
    position: Position,
    lhs: &mut ValueId,
    rhs: ValueId,
    existing_parameters: &BTreeMap<Position, ValueId>,
    active_parameters: &mut BTreeMap<Position, ValueId>,
    values: &mut ValueContext,
) {
    if *lhs == rhs {
        return;
    }
    let result = if let Some(&result) = active_parameters.get(&position) {
        result
    } else {
        let result = existing_parameters
            .get(&position)
            .copied()
            .unwrap_or_else(|| values.fresh());
        active_parameters.insert(position, result);
        result
    };

    *lhs = result;
}
