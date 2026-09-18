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
    pub(super) edges: Vec<AnalyzedEdge>,
    pub(super) output_frames: BTreeMap<Location, Frame>,
}

/// Analysis state for one location.
///
/// The structural CFG fixes a location's successor targets, so the analyzer
/// only adds entries to `contributions`; it never removes them. Therefore
/// `contributions` is non-empty if and only if `entry_frame` and `execution`
/// are both set.
#[derive(Debug, Default)]
pub(super) struct LocationState {
    pub(super) contributions: BTreeMap<Predecessor, Frame>,
    pub(super) entry_frame: Option<Frame>,
    pub(super) execution: Option<AnalyzedBlock>,
}
