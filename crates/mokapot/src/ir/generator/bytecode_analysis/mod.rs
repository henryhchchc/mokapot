//! Constructs provisional SSA while analyzing reachable JVM bytecode.

mod analysis;
mod frame;
pub(super) mod lifting;
mod values;

use self::analysis::Analyzer;
use crate::ir::generator::{draft::DraftMethod, error::Error};
pub use frame::FrameError;
use frame::{EntrySlots, Frame, Position, StackOperation, ValueCategory};

pub(super) fn analyze(cfg: &super::bytecode_cfg::Cfg<'_>) -> Result<DraftMethod, Error> {
    Analyzer::new(cfg)?.run()
}
