//! Internal state shared by block analysis stages.

use std::collections::BTreeMap;

use super::jvm;
use crate::{
    ir::{
        OperationKind, TerminatorKind, ValueId, control_flow::ControlTransfer,
        generator::bytecode_cfg,
    },
    jvm::code::ProgramCounter,
};

pub(super) type Frame = jvm::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Location {
    Bytecode(bytecode_cfg::JvmBlockId),
    /// The synthetic entry that installs the caught exception and enters a block.
    Handler(bytecode_cfg::JvmBlockId),
    Unwind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Predecessor {
    Entry,
    Location(Location),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PhiSite {
    pub(super) location: Location,
    pub(super) position: jvm::Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhiDefinition {
    pub(super) result: ValueId,
    pub(super) inputs: BTreeMap<Predecessor, ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AnalyzedEdge {
    pub(super) target: Location,
    pub(super) transfer: ControlTransfer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AnalyzedBlock {
    pub(super) caught_exception: Option<ValueId>,
    pub(super) operations: Vec<(ProgramCounter, OperationKind)>,
    pub(super) terminator: TerminatorKind,
    pub(super) terminator_source: Option<ProgramCounter>,
    pub(super) successors: AnalyzedSuccessors,
}

/// Successor transfers coupled to the frame contributed to each target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct AnalyzedSuccessors {
    pub(super) edges: Vec<AnalyzedEdge>,
    output_frames: BTreeMap<Location, Frame>,
}

impl AnalyzedSuccessors {
    pub(super) fn push(&mut self, edge: AnalyzedEdge, frame: Frame) {
        if let Some(existing) = self.output_frames.get(&edge.target) {
            assert_eq!(existing, &frame, "parallel edges must contribute one frame");
        } else {
            self.output_frames.insert(edge.target, frame);
        }
        self.edges.push(edge);
    }

    pub(super) fn take_output_frames(&mut self) -> BTreeMap<Location, Frame> {
        std::mem::take(&mut self.output_frames)
    }
}

/// Analysis state for one location.
///
/// The structural CFG fixes a location's successor targets, so the analyzer
/// only adds entries to `contributions`; it never removes them.
#[derive(Debug, Default)]
pub(super) struct LocationState {
    pub(super) contributions: BTreeMap<Predecessor, Frame>,
    pub(super) execution: LocationExecution,
}

/// Execution lifecycle of a reachable analysis location.
#[derive(Debug, Default)]
pub(super) enum LocationExecution {
    /// No input frame has yet been computed.
    #[default]
    Uninitialized,
    /// The input frame changed and the location must be executed.
    Pending { input: Frame },
    /// The location was executed with the current input frame.
    Complete { input: Frame, block: AnalyzedBlock },
}

impl LocationExecution {
    pub(super) const fn input(&self) -> Option<&Frame> {
        match self {
            Self::Uninitialized => None,
            Self::Pending { input } | Self::Complete { input, .. } => Some(input),
        }
    }

    pub(super) const fn block(&self) -> Option<&AnalyzedBlock> {
        match self {
            Self::Complete { block, .. } => Some(block),
            Self::Uninitialized | Self::Pending { .. } => None,
        }
    }

    pub(super) fn update_input(&mut self, input: Frame) -> bool {
        if self.input() == Some(&input) {
            return false;
        }
        *self = Self::Pending { input };
        true
    }

    pub(super) fn complete(&mut self, block: AnalyzedBlock) {
        let Self::Pending { input } = std::mem::take(self) else {
            unreachable!("only a pending location can finish execution");
        };
        *self = Self::Complete { input, block };
    }
}
