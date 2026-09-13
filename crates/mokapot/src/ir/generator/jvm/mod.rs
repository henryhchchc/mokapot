//! JVM-specific normalization, frame analysis, and instruction lifting.

pub(super) mod analysis;
pub(super) mod frame;
pub(super) mod instruction;
pub(super) mod lifting;
pub(super) mod normalization;

use crate::{ir::generator::error::MokaIRBuildError, jvm::Method};

use self::analysis::{AnalyzedJvmCfg, JvmFrameAnalyzer};

pub(super) fn analyze(method: &Method) -> Result<AnalyzedJvmCfg, MokaIRBuildError> {
    JvmFrameAnalyzer::for_method(method)?.analyze()
}
