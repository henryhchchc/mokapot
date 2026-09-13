mod entry;
mod error;
mod operations;
mod stack_frame;

#[cfg(test)]
mod tests;

pub(crate) use entry::Entry;
pub use error::JvmFrameError;
pub(crate) use operations::StackOperations;
pub(crate) use stack_frame::{CATEGORY_1, CATEGORY_2, Frame, Position};
