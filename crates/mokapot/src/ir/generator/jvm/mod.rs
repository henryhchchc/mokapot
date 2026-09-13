//! JVM-specific normalization, symbolic execution, and instruction lifting.

pub(super) mod frame;
pub(super) mod instruction;
pub(super) mod lifting;
pub(super) mod normalization;
pub(super) mod symbolic_execution;

use crate::{ir::generator::error::MokaIRBuildError, jvm::Method};

use self::symbolic_execution::{JvmSymbolicExecutor, SymbolicJvmCfg};

pub(super) fn build_symbolic_cfg(method: &Method) -> Result<SymbolicJvmCfg, MokaIRBuildError> {
    JvmSymbolicExecutor::for_method(method)?.analyze()
}
