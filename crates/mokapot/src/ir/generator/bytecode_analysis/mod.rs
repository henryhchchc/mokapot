//! Constructs a reachable register-form graph from JVM instructions.

mod analysis;
mod frame;
pub(super) mod lifting;
mod output;
mod values;

pub use frame::FrameError;
pub(super) use output::{PhiCandidate, ScalarBlock, ScalarGraph};

use self::analysis::Analyzer;
use crate::ir::generator::error::Error;
use frame::{EntrySlots, Frame, Position, StackOperation, ValueCategory};

pub(super) fn analyze(cfg: &super::bytecode_cfg::JvmBlockGraph<'_>) -> Result<ScalarGraph, Error> {
    Analyzer::new(cfg)?.run()
}
