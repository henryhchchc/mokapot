//! Block representation carrying analyzed frames on its successor arms.

use super::super::Frame;
use crate::ir::{BlockId, BlockKind, Operation, Terminator, control_flow::ControlTransfer};
use crate::{ir::generator::cfg::ArmKey, jvm::code::ProgramCounter};
use derive_more::Constructor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FrameSource {
    Entry,
    Predecessor { source: BlockId, arm: ArmKey },
}

#[derive(Debug, Clone, PartialEq, Eq, Constructor)]
pub(crate) struct FrameBlock {
    pub kind: BlockKind,
    pub operations: Vec<(ProgramCounter, Operation)>,
    pub terminator: FrameTerminator,
    pub terminator_source: Option<ProgramCounter>,
}

/// One analyzed outgoing arm of a frame-carrying terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FrameArm {
    /// A continuation into `target`, carrying the frame delivered there.
    Block {
        /// The arm's structural identity within its source.
        arm: ArmKey,
        /// The destination block.
        target: BlockId,
        /// The state transfer associated with this arm.
        transfer: ControlTransfer,
        /// The frame delivered to the destination.
        frame: Frame,
    },
    /// An exception escaping the method.
    Unwind {
        /// The arm's structural identity within its source.
        arm: ArmKey,
    },
}

pub(super) type FrameTerminator = Terminator<FrameArm>;

impl FrameBlock {
    /// The frames this block sends to its successors.
    pub(crate) fn outgoing_frames(
        &self,
        source: BlockId,
    ) -> impl Iterator<Item = (FrameSource, BlockId, &Frame)> {
        self.terminator
            .arms()
            .filter_map(move |frame_arm| match frame_arm {
                FrameArm::Block {
                    arm, target, frame, ..
                } => Some((
                    FrameSource::Predecessor { source, arm: *arm },
                    *target,
                    frame,
                )),
                FrameArm::Unwind { .. } => None,
            })
    }
}
