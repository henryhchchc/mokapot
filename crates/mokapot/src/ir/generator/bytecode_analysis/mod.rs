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

use self::{analyzer::Analyzer, jvm::Frame};
use crate::{
    ir::{ValueId, generator::error::Error},
    jvm::code::ProgramCounter,
};

pub(super) fn analyze(
    cfg: &super::bytecode_cfg::JvmBlockGraph<'_>,
) -> Result<scalar::ScalarGraph, Error> {
    Analyzer::new(cfg)?.run()
}

/// Symbolic executor used by block-level bytecode analysis.
struct Executor {
    definition_ids: BTreeMap<ProgramCounter, ValueId>,
    value_id_allocator: executor::ValueIdAllocator,
    receiver_value: Option<ValueId>,
    parameter_values: Vec<ValueId>,
    initial_frame: Frame,
}
