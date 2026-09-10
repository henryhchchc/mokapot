use std::fmt;

use super::ReturnAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display("%tmp{_0}")]
pub(super) struct ProvisionalValueId(u32);

impl ProvisionalValueId {
    pub(super) const fn new(index: u32) -> Self {
        Self(index)
    }

    pub(super) const fn index(self) -> u32 {
        self.0
    }
}

/// A scalar value used while discovering reachable JVM frame states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) enum DiscoveryValue {
    #[display("%this")]
    This,
    #[display("%arg{_0}")]
    Arg(u16),
    Local(ProvisionalValueId),
    #[display("%caught_exception{_0}")]
    CaughtException(ProvisionalValueId),
    #[display("%return_address")]
    ReturnAddress(ReturnAddress),
    #[display("%merged")]
    Merged,
    #[display("%invalid")]
    Invalid,
}

pub(super) trait FrameOperand:
    Clone + Eq + std::hash::Hash + fmt::Display + From<ProvisionalValueId> + From<ReturnAddress>
{
    fn return_address(&self) -> Option<ReturnAddress>;

    fn contains_return_address(&self) -> bool;
}

impl From<ProvisionalValueId> for DiscoveryValue {
    fn from(value: ProvisionalValueId) -> Self {
        Self::Local(value)
    }
}

impl From<ReturnAddress> for DiscoveryValue {
    fn from(value: ReturnAddress) -> Self {
        Self::ReturnAddress(value)
    }
}

impl FrameOperand for DiscoveryValue {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub(super) enum ScalarValue {
    #[display("{_0}")]
    Value(ProvisionalValueId),
    #[display("%return_address")]
    ReturnAddress(ReturnAddress),
}

impl From<ProvisionalValueId> for ScalarValue {
    fn from(value: ProvisionalValueId) -> Self {
        Self::Value(value)
    }
}

impl From<ReturnAddress> for ScalarValue {
    fn from(value: ReturnAddress) -> Self {
        Self::ReturnAddress(value)
    }
}

impl FrameOperand for ScalarValue {
    fn return_address(&self) -> Option<ReturnAddress> {
        match self {
            Self::ReturnAddress(address) => Some(*address),
            Self::Value(_) => None,
        }
    }

    fn contains_return_address(&self) -> bool {
        matches!(self, Self::ReturnAddress(_))
    }
}

impl crate::analysis::fixed_point::JoinSemiLattice for DiscoveryValue {
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

impl PartialOrd for DiscoveryValue {
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
