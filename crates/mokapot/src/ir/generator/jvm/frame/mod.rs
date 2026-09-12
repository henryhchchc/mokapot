mod entry;
mod error;
mod operations;
mod stack_frame;

#[cfg(test)]
mod tests;

pub(in crate::ir::generator) use entry::Entry;
pub use error::ExecutionError;
pub(in crate::ir::generator) use operations::StackOperations;
pub(in crate::ir::generator) use stack_frame::{DUAL_SLOT, FrameSlot, JvmStackFrame, SINGLE_SLOT};
