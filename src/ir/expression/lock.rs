use std::fmt::Formatter;

use std::fmt::Display;

use super::super::Argument;

/// An operation on a lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockOperation {
    /// Acquires the lock.
    Acquire(Argument),
    /// Releases the lock.
    Release(Argument),
}

impl Display for LockOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Acquire(lock) => write!(f, "acquire {lock}"),
            Self::Release(lock) => write!(f, "release {lock}"),
        }
    }
}
