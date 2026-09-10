use super::super::{FrameOperand, ReturnAddress, SsaValueId};

/// The abstract state of an operand during JVM frame analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(in crate::ir::generator) enum OperandState {
    #[display("%this")]
    This,
    #[display("%arg{_0}")]
    Arg(u16),
    Local(SsaValueId),
    #[display("%caught_exception{_0}")]
    CaughtException(SsaValueId),
    #[display("%return_address")]
    ReturnAddress(ReturnAddress),
    #[display("%merged")]
    Merged,
    #[display("%invalid")]
    Invalid,
}

impl From<SsaValueId> for OperandState {
    fn from(value: SsaValueId) -> Self {
        Self::Local(value)
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
    fn join(self, other: Self) -> Self {
        if self == other {
            return self;
        }
        match (self, other) {
            (Self::Invalid | Self::ReturnAddress(_), _)
            | (_, Self::Invalid | Self::ReturnAddress(_)) => Self::Invalid,
            _ => Self::Merged,
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
                (
                    Self::Merged,
                    Self::This | Self::Arg(_) | Self::Local(_) | Self::CaughtException(_),
                )
                | (Self::Invalid, _) => Some(Greater),
                (
                    Self::This | Self::Arg(_) | Self::Local(_) | Self::CaughtException(_),
                    Self::Merged,
                )
                | (_, Self::Invalid) => Some(Less),
                _ => None,
            }
        }
    }
}
