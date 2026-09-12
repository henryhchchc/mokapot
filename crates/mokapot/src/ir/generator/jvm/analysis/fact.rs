use std::{cmp::Ordering, collections::BTreeMap};

use crate::{
    analysis::fixed_point::{DataflowOutput, JoinSemiLattice},
    ir::{
        control_flow::ControlTransfer,
        generator::{
            identity::SsaValueId,
            jvm::{
                frame::{FrameSlot, JvmStackFrame},
                instruction::Instruction,
                lifting::frame_operand::FrameOperand,
                normalization::{Location, ReturnAddress},
            },
        },
    },
};

/// A stable identity for a frame value merged at a JVM location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(in crate::ir::generator) struct MergeIdentity {
    pub location: Location,
    pub slot: FrameSlot,
}

/// The abstract state of an operand during JVM frame analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(in crate::ir::generator) enum OperandState {
    Value(SsaValueId),
    #[display("%return_address")]
    ReturnAddress(ReturnAddress),
    #[display("%merged")]
    Merged(MergeIdentity),
    #[display("%invalid")]
    Invalid,
}

impl From<SsaValueId> for OperandState {
    fn from(value: SsaValueId) -> Self {
        Self::Value(value)
    }
}

impl From<ReturnAddress> for OperandState {
    fn from(value: ReturnAddress) -> Self {
        Self::ReturnAddress(value)
    }
}

impl FrameOperand for OperandState {
    fn return_address(&self) -> Option<ReturnAddress> {
        match self {
            Self::ReturnAddress(address) => Some(*address),
            _ => None,
        }
    }

    fn contains_return_address(&self) -> bool {
        matches!(self, Self::ReturnAddress(_) | Self::Invalid)
    }
}

impl JoinSemiLattice for OperandState {
    fn join_assign(&mut self, other: Self) -> bool {
        if *self == other {
            return false;
        }
        let joined = Self::Invalid;
        if *self == joined {
            false
        } else {
            *self = joined;
            true
        }
    }
}

impl PartialOrd for OperandState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use Ordering::{Equal, Greater, Less};

        if self == other {
            Some(Equal)
        } else {
            match (self, other) {
                (Self::Invalid, _) => Some(Greater),
                (_, Self::Invalid) => Some(Less),
                _ => None,
            }
        }
    }
}

/// A frame tagged with the location at which its values are merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) struct JvmFrameFact {
    pub(super) location: Location,
    pub(super) frame: JvmStackFrame<OperandState>,
}

impl JvmFrameFact {
    pub(super) fn new(location: Location, frame: JvmStackFrame<OperandState>) -> Self {
        let frame = if matches!(location, Location::Unwind) {
            frame.erase_values()
        } else {
            frame
        };
        Self { location, frame }
    }

    pub(super) fn into_frame(self) -> JvmStackFrame<OperandState> {
        self.frame
    }
}

impl JoinSemiLattice for JvmFrameFact {
    fn join_assign(&mut self, other: Self) -> bool {
        assert_eq!(self.location, other.location);
        let location = self.location;
        self.frame
            .join_assign_values_with(other.frame, |slot, lhs, rhs| {
                if *lhs == rhs {
                    return false;
                }
                let merged = MergeIdentity { location, slot };
                let value = match (*lhs, rhs) {
                    (OperandState::Invalid | OperandState::ReturnAddress(_), _)
                    | (_, OperandState::Invalid | OperandState::ReturnAddress(_)) => {
                        OperandState::Invalid
                    }
                    (OperandState::Merged(identity), _) if identity == merged => return false,
                    _ => OperandState::Merged(merged),
                };
                if *lhs == value {
                    false
                } else {
                    *lhs = value;
                    true
                }
            })
    }
}

impl PartialOrd for JvmFrameFact {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use Ordering::{Equal, Greater, Less};

        if self.location != other.location {
            return None;
        }
        let mut lhs = self.clone();
        let mut rhs = other.clone();
        let lhs_changes = JoinSemiLattice::join_assign(&mut lhs, other.clone());
        let rhs_changes = JoinSemiLattice::join_assign(&mut rhs, self.clone());
        match (lhs_changes, rhs_changes) {
            (false, false) => Some(Equal),
            (false, true) => Some(Greater),
            (true, false) => Some(Less),
            (true, true) => None,
        }
    }
}

/// Transfer output for one reachable JVM location.
pub(in crate::ir::generator) struct JvmFlowOutput {
    pub(super) instruction: Instruction,
    pub(super) outgoing: Vec<JvmFlowOutgoing>,
}

pub(super) struct JvmFlowOutgoing {
    pub(super) target: Location,
    pub(super) transfer: ControlTransfer<OperandState>,
    pub(super) frame: JvmFrameFact,
}

impl DataflowOutput<Location, JvmFrameFact> for JvmFlowOutput {
    fn successors<'a>(&'a self) -> impl Iterator<Item = (&'a Location, &'a JvmFrameFact)>
    where
        Location: 'a,
        JvmFrameFact: 'a,
    {
        self.outgoing
            .iter()
            .map(|outgoing| (&outgoing.target, &outgoing.frame))
    }

    fn into_successors(self) -> impl Iterator<Item = (Location, JvmFrameFact)> {
        self.outgoing
            .into_iter()
            .map(|outgoing| (outgoing.target, outgoing.frame))
    }
}

/// One outgoing edge and its exact symbolic frame.
pub(in crate::ir::generator) struct JvmOutgoing {
    pub target: Location,
    pub transfer: ControlTransfer<OperandState>,
    pub frame: JvmStackFrame<OperandState>,
}

/// Completed abstract-execution facts for one reachable JVM location.
pub(in crate::ir::generator) struct AnalyzedLocation {
    pub incoming: JvmStackFrame<OperandState>,
    pub instruction: Instruction,
    pub outgoing: Vec<JvmOutgoing>,
    pub caught_exception: Option<SsaValueId>,
}

/// Reachable JVM locations and abstract control-flow facts.
pub(in crate::ir::generator) struct AnalyzedJvmCfg {
    pub entry_location: Location,
    /// The original frame entering the method.
    pub initial_frame: JvmStackFrame<OperandState>,
    pub locations: BTreeMap<Location, AnalyzedLocation>,
    pub phi_values: BTreeMap<MergeIdentity, SsaValueId>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}
