use super::{FrameOperand, ReturnAddress};

use crate::ir::generator::SsaValueId;

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
