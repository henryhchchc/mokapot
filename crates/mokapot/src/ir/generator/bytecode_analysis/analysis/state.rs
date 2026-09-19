//! Internal state shared by block analysis stages.

use std::collections::BTreeMap;

use super::{Frame, Position};
use crate::{
    ir::{BlockId, EdgeId, OperationKind, TerminatorKind, ValueId, control_flow::ControlTransfer},
    jvm::code::ProgramCounter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Predecessor {
    Entry,
    Block(BlockId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PhiSite {
    pub block: BlockId,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PhiDefinition {
    pub result: ValueId,
    pub inputs: BTreeMap<Predecessor, ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiftedEdge {
    pub id: EdgeId,
    pub target: BlockId,
    pub transfer: ControlTransfer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiftedBlock {
    pub caught_exception: Option<ValueId>,
    pub operations: Vec<(ProgramCounter, OperationKind)>,
    pub terminator: TerminatorKind,
    pub terminator_source: Option<ProgramCounter>,
    pub successors: LiftedSuccessors,
}

/// Successor transfers coupled to the frame contributed to each target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LiftedSuccessors {
    pub edges: Vec<LiftedEdge>,
    output_frames: BTreeMap<BlockId, Frame>,
}

impl LiftedSuccessors {
    pub(crate) fn push(&mut self, edge: LiftedEdge, frame: Frame) {
        if let Some(existing) = self.output_frames.get(&edge.target) {
            assert_eq!(existing, &frame, "parallel edges must contribute one frame");
        } else {
            self.output_frames.insert(edge.target, frame);
        }
        self.edges.push(edge);
    }

    pub(crate) fn take_output_frames(&mut self) -> BTreeMap<BlockId, Frame> {
        std::mem::take(&mut self.output_frames)
    }
}

/// Analysis state for one normalized block.
///
/// The structural CFG fixes predecessor and successor membership. The analyzer
/// only fills frame `contributions`; it never changes topology.
#[derive(Debug, Default)]
pub(crate) struct BlockState {
    pub contributions: BTreeMap<Predecessor, Frame>,
    pub execution: BlockExecution,
}

/// Execution lifecycle of a reachable normalized block.
#[derive(Debug, Default)]
pub(crate) enum BlockExecution {
    /// No input frame has yet been computed.
    #[default]
    Uninitialized,
    /// The input frame changed and the block must be executed.
    Pending { input: Frame },
    /// The block was executed with the current input frame.
    Complete { input: Frame, block: LiftedBlock },
}

/// The complete analysis state passed to scalar-graph materialization.
pub(crate) struct CompletedAnalysis {
    pub blocks: BTreeMap<BlockId, BlockState>,
    pub phi_definitions: BTreeMap<PhiSite, PhiDefinition>,
    pub receiver_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}

impl BlockExecution {
    pub(crate) const fn input(&self) -> Option<&Frame> {
        match self {
            Self::Uninitialized => None,
            Self::Pending { input } | Self::Complete { input, .. } => Some(input),
        }
    }

    pub(crate) fn update_input(&mut self, input: Frame) -> bool {
        if self.input() == Some(&input) {
            return false;
        }
        *self = Self::Pending { input };
        true
    }

    pub(crate) fn complete(&mut self, block: LiftedBlock) {
        let Self::Pending { input } = std::mem::take(self) else {
            unreachable!("only a pending block can finish execution");
        };
        *self = Self::Complete { input, block };
    }
}
