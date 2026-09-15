//! JVM-specific frame modeling and symbolic execution.

pub(super) mod frame;
pub(super) mod symbolic_execution;

use crate::{ir::generator::error::Error, jvm::Method};

pub(super) fn build_symbolic_cfg(method: &Method) -> Result<symbolic_execution::Cfg, Error> {
    symbolic_execution::Executor::for_method(method)?.execute()
}
