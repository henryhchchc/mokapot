//! Constructs a reachable register-form graph from JVM instructions.

mod block;
mod execution;
pub(super) mod fallibility;
mod instruction;
pub(super) mod jvm;
pub(super) mod lifting;

use std::collections::BTreeMap;

pub(super) use instruction::RegisterInstruction;

use crate::{
    ir::generator::{bytecode_analysis::jvm::Frame, error::Error, identity::SsaValueId},
    jvm::{Method, code::MethodBody},
};

pub(super) fn analyze(
    method: &Method,
    cfg: &super::bytecode_cfg::BytecodeCfg,
) -> Result<super::ssa::UnfinalizedGraph, Error> {
    block::analyze(method, cfg)
}

/// Symbolic executor used by block-level bytecode analysis.
struct Executor<'method> {
    body: &'method MethodBody,
    definition_ids: BTreeMap<crate::jvm::code::ProgramCounter, SsaValueId>,
    value_id_allocator: execution::ValueIdAllocator,
    receiver_value: Option<SsaValueId>,
    parameter_values: Vec<SsaValueId>,
    initial_frame: Frame<Value>,
}

/// An abstract JVM frame value while analyzing structural blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) enum Value {
    Ssa(SsaValueId),
    #[display("%return_address")]
    ReturnAddress(SsaValueId),
    #[display("%invalid")]
    Invalid,
}

impl From<SsaValueId> for Value {
    fn from(value: SsaValueId) -> Self {
        Self::Ssa(value)
    }
}
