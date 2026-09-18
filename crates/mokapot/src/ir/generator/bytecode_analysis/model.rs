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
    frames: BTreeMap<Location, Frame>,
}

impl AnalyzedSuccessors {
    pub(super) fn push(&mut self, edge: AnalyzedEdge, frame: Frame) {
        if let Some(existing) = self.frames.get(&edge.target) {
            assert_eq!(existing, &frame, "parallel edges must contribute one frame");
        } else {
            self.frames.insert(edge.target, frame);
        }
        self.edges.push(edge);
    }

    pub(super) fn take_frames(&mut self) -> BTreeMap<Location, Frame> {
        std::mem::take(&mut self.frames)
    }
}

/// Analysis state for one location.
///
/// The structural CFG fixes a location's successor targets, so the analyzer
/// only adds entries to `contributions`; it never removes them.
#[derive(Debug, Default)]
pub(super) struct LocationState {
    pub(super) contributions: BTreeMap<Predecessor, Frame>,
    pub(super) analysis: LocationAnalysis,
}

/// Lifecycle of a reachable analysis location.
#[derive(Debug, Default)]
pub(super) enum LocationAnalysis {
    /// No contribution has yet been merged.
    #[default]
    Unreached,
    /// The entry frame changed and the location must be executed.
    Pending(Frame),
    /// The location was executed with the current entry frame.
    Executed { entry: Frame, block: AnalyzedBlock },
}

impl LocationAnalysis {
    pub(super) const fn entry_frame(&self) -> Option<&Frame> {
        match self {
            Self::Unreached => None,
            Self::Pending(frame) | Self::Executed { entry: frame, .. } => Some(frame),
        }
    }

    pub(super) const fn execution(&self) -> Option<&AnalyzedBlock> {
        match self {
            Self::Executed { block, .. } => Some(block),
            Self::Unreached | Self::Pending(_) => None,
        }
    }

    pub(super) fn update_entry(&mut self, frame: Frame) -> bool {
        if self.entry_frame() == Some(&frame) {
            return false;
        }
        *self = Self::Pending(frame);
        true
    }

    pub(super) fn finish(&mut self, block: AnalyzedBlock) {
        let Self::Pending(entry) = std::mem::take(self) else {
            unreachable!("only a pending location can finish execution");
        };
        *self = Self::Executed { entry, block };
    }
}
