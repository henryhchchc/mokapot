//! Internal state shared by block analysis stages.

use std::collections::HashMap;

use super::{Frame, Position};
use crate::{
    ir::{
        BlockId, BlockKind, EdgeId, Operation, Terminator, ValueId, control_flow::ControlTransfer,
    },
    jvm::code::ProgramCounter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Contribution {
    Entry,
    Edge(EdgeId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ParameterSite {
    pub block: BlockId,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LiftedEdge {
    Block {
        id: EdgeId,
        target: BlockId,
        transfer: ControlTransfer,
    },
    Unwind {
        id: EdgeId,
    },
}

impl LiftedEdge {
    pub(super) const fn id(&self) -> EdgeId {
        match self {
            Self::Block { id, .. } | Self::Unwind { id } => *id,
        }
    }

    pub(super) const fn block_target(&self) -> Option<BlockId> {
        match self {
            Self::Block { target, .. } => Some(*target),
            Self::Unwind { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LiftedBlock {
    pub kind: BlockKind,
    pub operations: Vec<(ProgramCounter, Operation)>,
    pub terminator: LiftedTerminator,
    pub terminator_source: Option<ProgramCounter>,
}

pub(super) type LiftedArm = (LiftedEdge, Option<Frame>);

pub(super) type LiftedTerminator = Terminator<LiftedArm>;

/// Analysis state for one normalized block.
///
/// The structural CFG fixes predecessor and successor membership. The analyzer
/// only fills frame `contributions`; it never changes topology.
#[derive(Debug, Default)]
pub(super) struct BlockState {
    pub contributions: HashMap<Contribution, Frame>,
    pub execution: BlockExecution,
}

/// Execution lifecycle of a reachable normalized block.
#[derive(Debug, Default)]
pub(super) enum BlockExecution {
    /// No input frame has yet been computed.
    #[default]
    Uninitialized,
    /// The input frame changed and the block must be executed.
    Pending { input: Frame },
    /// The block was executed with the current input frame.
    Complete {
        input: Frame,
        block: Box<LiftedBlock>,
    },
}

/// The complete analysis state passed to draft-IR materialization.
pub(super) struct CompletedAnalysis {
    pub blocks: HashMap<BlockId, BlockState>,
    pub parameter_definitions: HashMap<ParameterSite, ValueId>,
    pub receiver_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}

impl BlockExecution {
    pub(super) const fn input(&self) -> Option<&Frame> {
        match self {
            Self::Uninitialized => None,
            Self::Pending { input } | Self::Complete { input, .. } => Some(input),
        }
    }

    pub(super) fn update_input(&mut self, input: Frame) -> bool {
        if self.input() == Some(&input) {
            return false;
        }
        *self = Self::Pending { input };
        true
    }

    pub(super) fn complete(&mut self, block: LiftedBlock) {
        let Self::Pending { input } = std::mem::take(self) else {
            unreachable!("only a pending block can finish execution");
        };
        *self = Self::Complete {
            input,
            block: Box::new(block),
        };
    }
}
