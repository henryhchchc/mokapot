use super::super::{FrameOperand, Location, ReturnAddress, SsaValueId, jvm_frame::FrameSlot};

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

impl crate::analysis::fixed_point::JoinSemiLattice for OperandState {
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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::{Equal, Greater, Less};

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
