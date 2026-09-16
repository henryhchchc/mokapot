//! Constructs a reachable register-form graph from JVM instructions.

mod analyzer;
mod block_execution;
mod executor;
mod instruction;
pub(super) mod jvm;
pub(super) mod lifting;
mod materialize;
mod model;
mod phis;
pub(super) mod scalar;

use std::collections::BTreeMap;

pub(super) use instruction::LiftedEffect;

use crate::{
    ir::generator::{
        bytecode_analysis::jvm::Frame,
        error::{Error, MalformedBytecode},
        identity::SsaValueId,
    },
    jvm::{Method, code::MethodBody},
};

pub(super) fn analyze(
    method: &Method,
    cfg: &super::bytecode_cfg::BytecodeCfg,
) -> Result<scalar::ScalarGraph, Error> {
    analyzer::analyze(method, cfg)
}

/// Symbolic executor used by block-level bytecode analysis.
struct Executor<'method> {
    body: &'method MethodBody,
    definition_ids: BTreeMap<crate::jvm::code::ProgramCounter, SsaValueId>,
    value_id_allocator: executor::ValueIdAllocator,
    receiver_value: Option<SsaValueId>,
    parameter_values: Vec<SsaValueId>,
    initial_frame: Frame<FrameValue>,
}

/// An abstract JVM frame value while analyzing structural blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) enum FrameValue {
    Ordinary(SsaValueId),
    #[display("%return_address")]
    ReturnAddress(SsaValueId),
    #[display("%invalid")]
    Invalid,
}

impl FrameValue {
    /// Extracts the temporary ID from either well-formed scalar frame value.
    ///
    /// Phi inputs may carry return-address values; ordinary scalar IR may not.
    pub(super) const fn into_ssa_value_id(self) -> Result<SsaValueId, Error> {
        match self {
            Self::Ordinary(value) | Self::ReturnAddress(value) => Ok(value),
            Self::Invalid => Err(Error::malformed(None, MalformedBytecode::InvalidFrameValue)),
        }
    }

    /// Extracts the temporary ID from an ordinary scalar frame value.
    pub(super) const fn into_ordinary_ssa_value_id(self) -> Result<SsaValueId, Error> {
        match self {
            Self::Ordinary(value) => Ok(value),
            Self::ReturnAddress(_) | Self::Invalid => {
                Err(Error::malformed(None, MalformedBytecode::InvalidFrameValue))
            }
        }
    }
}

impl From<SsaValueId> for FrameValue {
    fn from(value: SsaValueId) -> Self {
        Self::Ordinary(value)
    }
}
