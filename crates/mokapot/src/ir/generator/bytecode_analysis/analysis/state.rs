//! Internal state shared by block analysis stages.

use std::collections::BTreeMap;

use super::{Frame, Position};
use crate::{
    ir::{
        OperationKind, TerminatorKind, ValueId, control_flow::ControlTransfer,
        generator::bytecode_cfg,
    },
    jvm::code::ProgramCounter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Location {
    Bytecode(bytecode_cfg::JvmBlockId),
    /// The synthetic entry that installs the caught exception and enters a block.
    Handler(bytecode_cfg::JvmBlockId),
    Unwind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Predecessor {
    Entry,
    Location(Location),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PhiSite {
    pub location: Location,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PhiDefinition {
    pub result: ValueId,
    pub inputs: BTreeMap<Predecessor, ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiftedEdge {
    pub target: Location,
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
    output_frames: BTreeMap<Location, Frame>,
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

    pub(crate) fn take_output_frames(&mut self) -> BTreeMap<Location, Frame> {
        std::mem::take(&mut self.output_frames)
    }
}

/// Analysis state for one location.
///
/// The structural CFG fixes a location's successor targets, so the analyzer
/// only adds entries to `contributions`; it never removes them.
#[derive(Debug, Default)]
pub(crate) struct LocationState {
    pub contributions: BTreeMap<Predecessor, Frame>,
    pub execution: LocationExecution,
}

/// Execution lifecycle of a reachable analysis location.
#[derive(Debug, Default)]
pub(crate) enum LocationExecution {
    /// No input frame has yet been computed.
    #[default]
    Uninitialized,
    /// The input frame changed and the location must be executed.
    Pending { input: Frame },
    /// The location was executed with the current input frame.
    Complete { input: Frame, block: LiftedBlock },
}

/// The complete analysis state passed to scalar-graph materialization.
pub(crate) struct CompletedAnalysis {
    pub locations: BTreeMap<Location, LocationState>,
    pub phi_definitions: BTreeMap<PhiSite, PhiDefinition>,
    pub receiver_value: Option<ValueId>,
    pub parameter_values: Vec<ValueId>,
}

impl LocationExecution {
    pub(crate) const fn input(&self) -> Option<&Frame> {
        match self {
            Self::Uninitialized => None,
            Self::Pending { input } | Self::Complete { input, .. } => Some(input),
        }
    }

    pub(crate) const fn block(&self) -> Option<&LiftedBlock> {
        match self {
            Self::Complete { block, .. } => Some(block),
            Self::Uninitialized | Self::Pending { .. } => None,
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
            unreachable!("only a pending location can finish execution");
        };
        *self = Self::Complete { input, block };
    }
}
