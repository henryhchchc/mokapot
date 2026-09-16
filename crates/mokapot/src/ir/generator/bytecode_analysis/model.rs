//! Internal state shared by block analysis stages.

use std::collections::BTreeMap;

use super::{FrameValue, jvm};
use crate::{
    ir::generator::{bytecode_cfg, identity::SsaValueId},
    ir::{OperationKind, TerminatorKind, control_flow::ControlTransfer},
    jvm::code::ProgramCounter,
};

pub(super) type Frame = jvm::Frame<FrameValue>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Location {
    Bytecode(bytecode_cfg::StructuralBlockId),
    Handler(bytecode_cfg::HandlerId),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PhiKind {
    Ordinary,
    ReturnAddress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhiDefinition {
    pub(super) result: SsaValueId,
    pub(super) kind: PhiKind,
    pub(super) inputs: BTreeMap<Predecessor, SsaValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AnalyzedSuccessor {
    pub(super) target: Location,
    pub(super) transfer: ControlTransfer<FrameValue>,
    pub(super) frame: Frame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AnalyzedBlock {
    pub(super) caught_exception: Option<SsaValueId>,
    pub(super) operations: Vec<(ProgramCounter, OperationKind<FrameValue>)>,
    pub(super) terminator: TerminatorKind<FrameValue>,
    pub(super) terminator_source: Option<ProgramCounter>,
    pub(super) successors: Vec<AnalyzedSuccessor>,
}

#[derive(Debug, Default)]
pub(super) struct LocationState {
    pub(super) contributions: BTreeMap<Predecessor, Frame>,
    pub(super) entry_frame: Option<Frame>,
    pub(super) execution: Option<AnalyzedBlock>,
    pub(super) outgoing_frames: BTreeMap<Location, Frame>,
}
