//! JVM-specific normalization, symbolic execution, and instruction lifting.

pub(super) mod frame;
pub(super) mod instruction;
pub(super) mod lifting;
pub(super) mod normalization;
pub(super) mod symbolic_execution;

use crate::{ir::generator::error::MokaIRBuildError, jvm::Method};

pub(super) fn build_symbolic_cfg(
    method: &Method,
) -> Result<symbolic_execution::Cfg, MokaIRBuildError> {
    symbolic_execution::Executor::for_method(method)?.execute()
}
