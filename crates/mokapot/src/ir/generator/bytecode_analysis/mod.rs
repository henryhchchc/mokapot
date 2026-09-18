//! Constructs a reachable register-form graph from JVM instructions.

mod analyzer;
mod block_execution;
mod executor;
pub(super) mod jvm;
pub(super) mod lifting;
mod materialize;
mod model;
mod phis;
mod scalar;

pub(super) use scalar::{PhiCandidate, ScalarBlock, ScalarGraph, Successor};

use std::collections::BTreeMap;

use crate::{
    ir::{
        ValueId,
        generator::{
            bytecode_analysis::{analyzer::Analyzer, jvm::Frame},
            error::Error,
        },
    },
    jvm::{Method, code::MethodBody},
};

pub(super) fn analyze(
    method: &Method,
    cfg: &super::bytecode_cfg::BytecodeCfg,
) -> Result<scalar::ScalarGraph, Error> {
    Analyzer::new(method, cfg)?.run()
}

/// Symbolic executor used by block-level bytecode analysis.
struct Executor<'method> {
    body: &'method MethodBody,
    definition_ids: BTreeMap<crate::jvm::code::ProgramCounter, ValueId>,
    value_id_allocator: executor::ValueIdAllocator,
    receiver_value: Option<ValueId>,
    parameter_values: Vec<ValueId>,
    initial_frame: Frame,
}
