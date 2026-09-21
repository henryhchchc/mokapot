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
    parameters: BlockParameters,
    /// The most recently merged input frame.
    input: Frame,
    /// The interpretation of `input`, once the block has run with it.
    result: Option<FrameBlock>,
}

impl BlockState {
    pub(super) fn new(source: FrameSource, frame: Frame) -> Self {
        Self {
            incoming_frames: HashMap::from([(source, frame.clone())]),
            parameters: BlockParameters::default(),
            input: frame,
            result: None,
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
        let input = self
            .parameters
            .merge(self.incoming_frames.values(), block_pc, values)?;
        Ok(self.update_input(input))
    }

    /// The frame this block is to be interpreted with.
    pub(super) fn input(&self) -> Frame {
        self.input.clone()
    }

    pub(super) fn complete(&mut self, block: FrameBlock) {
        debug_assert!(
            self.result.is_none(),
            "only a block awaiting interpretation can finish interpretation"
        );
        self.result = Some(block);
    }

    pub(super) fn into_solution(self) -> BlockSolution {
        let block = self
            .result
            .expect("the worklist must drain only after every reachable block completes");
        BlockSolution::new(self.incoming_frames, self.parameters.declared, block)
    }

    /// Records a freshly merged input frame, reporting whether it differs from
    /// the frame the block last executed with.
    fn update_input(&mut self, input: Frame) -> bool {
        if self.input == input {
            return false;
        }
        self.input = input;
        self.result = None;
        true
    }
}

/// The block parameters of one block.
///
/// A site is declared a parameter only while the merged frame still holds a
/// value for it, but its identity is retained for the rest of the analysis.
/// Reusing the identity lets the merge reach a fixed point; allocating a fresh
/// one on every reappearance instead makes a cyclic block churn forever.
#[derive(Debug, Default)]
struct BlockParameters {
    /// Sites whose values the block currently declares on entry.
    declared: BTreeMap<Position, ValueId>,
    /// Identity retained for every site that has ever been a parameter.
    identities: BTreeMap<Position, ValueId>,
}

impl BlockParameters {
    /// Merges every incoming frame, updating the declared parameters in place.
    fn merge<'frames>(
        &mut self,
        frames: impl Iterator<Item = &'frames Frame>,
        block_pc: Option<ProgramCounter>,
        values: &mut ValueContext,
    ) -> Result<Frame, Error> {
        let mut frames = frames;
        let mut merged = frames
            .next()
            .expect("a block must be created with its first incoming frame")
            .clone();
        // A site that is already a parameter stays one and keeps its identity,
        // even while every incoming frame transiently agrees. That monotone
        // parameter set is what lets a cyclic block reach a fixed point instead
        // of flipping between a parameter and a narrower frame; `canonicalize`
        // eliminates the parameters a block does not end up needing.
        let mut active = self.declared.clone();

        for incoming in frames {
            merged
                .merge_from_with(incoming.clone(), |position, lhs, rhs| {
                    self.join(position, lhs, rhs, &mut active, values);
                    Ok::<_, Error>(())
                })
                .map_err(|error| match block_pc {
                    Some(pc) => error.at_instruction(pc),
                    None => error,
                })?;
        }

        self.declared = active
            .into_iter()
            .filter(|(position, result)| merged.value_at(*position) == Some(result))
            .collect();
        Ok(merged)
    }

    /// Records one merged position, keeping an active parameter as is and
    /// otherwise reusing or allocating the site's identity.
    fn join(
        &mut self,
        position: Position,
        lhs: &mut ValueId,
        rhs: ValueId,
        active: &mut BTreeMap<Position, ValueId>,
        values: &mut ValueContext,
    ) {
        if let Some(&result) = active.get(&position) {
            *lhs = result;
            return;
        }
        if *lhs == rhs {
            return;
        }
        let result = *self
            .identities
            .entry(position)
            .or_insert_with(|| values.fresh());
        active.insert(position, result);
        *lhs = result;
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
