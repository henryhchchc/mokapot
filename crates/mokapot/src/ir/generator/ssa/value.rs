use super::{FrameOperand, ReturnAddress};

/// An SSA identity used internally before public identities are emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display("%ssa{_0}")]
pub(in crate::ir::generator) struct SsaValueId(u32);

impl SsaValueId {
    pub(in crate::ir::generator) const fn new(index: u32) -> Self {
        Self(index)
    }

    pub(in crate::ir::generator) const fn index(self) -> u32 {
        self.0
    }
}

/// An exact value inhabiting a JVM frame during SSA construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub(in crate::ir::generator) enum SsaFrameValue {
    #[display("{_0}")]
    Value(SsaValueId),
    #[display("%return_address")]
    ReturnAddress(ReturnAddress),
}

impl From<SsaValueId> for SsaFrameValue {
    fn from(value: SsaValueId) -> Self {
        Self::Value(value)
    }
}

impl From<ReturnAddress> for SsaFrameValue {
    fn from(value: ReturnAddress) -> Self {
        Self::ReturnAddress(value)
    }
}

impl FrameOperand for SsaFrameValue {
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
