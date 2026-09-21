//! Mutable frame merging and execution state for one block.

use std::collections::{BTreeMap, HashMap};

use super::{
    super::{Frame, Position, values::ValueContext},
    frame_block::{FrameBlock, FrameSource},
};
use crate::{
    ir::{ValueId, generator::error::Error},
    jvm::code::ProgramCounter,
};

/// Mutable analysis state for one reachable block.
#[derive(Debug)]
pub(super) struct BlockState {
    incoming_frames: HashMap<FrameSource, Frame>,
    parameters: BTreeMap<Position, ValueId>,
    /// Every parameter identity ever allocated at this block, keyed by site.
    ///
    /// Identities are retained across merges so that a site which stops and then
    /// resumes conflicting reuses the same value instead of allocating a fresh
    /// one; otherwise merging a cyclic block churns through new identities
    /// without ever reaching a fixed point.
    parameter_ids: BTreeMap<Position, ValueId>,
    /// The most recently merged input frame.
    input: Frame,
    execution: ExecutionState,
}

impl BlockState {
    pub(super) fn new(source: FrameSource, frame: Frame) -> Self {
        Self {
            incoming_frames: HashMap::from([(source, frame.clone())]),
            parameters: BTreeMap::new(),
            parameter_ids: BTreeMap::new(),
            input: frame,
            execution: ExecutionState::Ready,
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
            &mut self.parameter_ids,
            block_pc,
            values,
        )?;
        self.parameters = parameters;
        Ok(self.update_input(input))
    }

    pub(super) fn begin_execution(&mut self) -> Frame {
        self.execution.begin();
        self.input.clone()
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

    /// Records a freshly merged input frame, reporting whether it differs from
    /// the frame the block last executed with.
    fn update_input(&mut self, input: Frame) -> bool {
        if self.input == input {
            return false;
        }
        self.input = input;
        self.execution = ExecutionState::Ready;
        true
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
    Ready,
    Running,
    Complete(FrameBlock),
}

impl ExecutionState {
    fn begin(&mut self) {
        let Self::Ready = std::mem::replace(self, Self::Running) else {
            panic!("a scheduled block must be ready");
        };
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
    previous_parameters: &BTreeMap<Position, ValueId>,
    parameter_ids: &mut BTreeMap<Position, ValueId>,
    block_pc: Option<ProgramCounter>,
    values: &mut ValueContext,
) -> Result<(Frame, BTreeMap<Position, ValueId>), Error> {
    let mut frames = frames;
    let mut merged = frames
        .next()
        .expect("a block must be created with its first incoming frame")
        .clone();
    // A site that is already a parameter stays one and keeps its identity, even
    // while every incoming frame transiently agrees. Making the parameter set
    // monotone is what lets a cyclic block reach a fixed point instead of
    // flipping between a parameter and a narrower frame. `canonicalize`
    // eliminates the parameters a block does not end up needing.
    let mut active_parameters = previous_parameters.clone();

    for incoming in frames {
        merged
            .merge_from_with(incoming.clone(), |position, lhs, rhs| {
                merge_value(
                    position,
                    lhs,
                    rhs,
                    parameter_ids,
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

    // A site is declared a parameter only while the merged frame still holds
    // its identity; the identity itself is retained in `parameter_ids` for
    // later merges.
    let declared = active_parameters
        .into_iter()
        .filter(|(position, result)| merged.value_at(*position) == Some(result))
        .collect();

    Ok((merged, declared))
}

fn merge_value(
    position: Position,
    lhs: &mut ValueId,
    rhs: ValueId,
    parameter_ids: &mut BTreeMap<Position, ValueId>,
    active_parameters: &mut BTreeMap<Position, ValueId>,
    values: &mut ValueContext,
) {
    if let Some(&result) = active_parameters.get(&position) {
        *lhs = result;
        return;
    }
    if *lhs == rhs {
        return;
    }
    let result = *parameter_ids
        .entry(position)
        .or_insert_with(|| values.fresh());
    active_parameters.insert(position, result);
    *lhs = result;
}
