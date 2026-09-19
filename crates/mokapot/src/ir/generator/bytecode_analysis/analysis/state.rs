//! Internal state shared by block analysis stages.

use std::collections::BTreeMap;

use super::{Frame, Position};
use crate::{
    ir::{
        BlockId, BlockKind, EdgeId, OperationKind, SuccessorTarget, TerminatorKind, ValueId,
        control_flow::ControlTransfer,
    },
    jvm::code::ProgramCounter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Contribution {
    Entry,
    Edge(EdgeId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ParameterSite {
    pub block: BlockId,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParameterDefinition {
    pub result: ValueId,
    pub inputs: BTreeMap<Contribution, ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiftedEdge {
    pub id: EdgeId,
    pub target: SuccessorTarget,
    pub transfer: ControlTransfer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiftedBlock {
    pub kind: BlockKind,
    pub operations: Vec<(ProgramCounter, OperationKind)>,
    pub terminator: TerminatorKind,
    pub terminator_source: Option<ProgramCounter>,
    pub successors: LiftedSuccessors,
}

/// Successor transfers coupled to the frame contributed to each target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LiftedSuccessors {
    pub edges: Vec<(LiftedEdge, Option<Frame>)>,
}

impl LiftedSuccessors {
    pub(crate) fn push(&mut self, edge: LiftedEdge, frame: Frame) {
        self.push_optional(edge, Some(frame));
    }

    pub(crate) fn push_optional(&mut self, edge: LiftedEdge, frame: Option<Frame>) {
        debug_assert!(
            self.edges
                .iter()
                .all(|(existing, _)| existing.id != edge.id)
        );
        self.edges.push((edge, frame));
    }
}

/// Analysis state for one normalized block.
///
/// The structural CFG fixes predecessor and successor membership. The analyzer
/// only fills frame `contributions`; it never changes topology.
#[derive(Debug, Default)]
pub(crate) struct BlockState {
    pub contributions: BTreeMap<Contribution, Frame>,
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

/// The complete analysis state passed to draft-IR materialization.
pub(crate) struct CompletedAnalysis {
    pub blocks: BTreeMap<BlockId, BlockState>,
    pub parameter_definitions: BTreeMap<ParameterSite, ParameterDefinition>,
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
