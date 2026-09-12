mod entry;
mod error;
mod operations;
mod stack_frame;

#[cfg(test)]
mod tests;

pub(crate) use entry::Entry;
pub use error::ExecutionError;
pub(crate) use operations::StackOperations;
pub(crate) use stack_frame::{DUAL_SLOT, FrameSlot, JvmStackFrame, SINGLE_SLOT};
