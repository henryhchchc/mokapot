use std::fmt::Formatter;

use std::fmt::Display;

use super::super::Argument;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockOperation {
    Acquire(Argument),
    Release(Argument),
}

impl Display for LockOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use LockOperation::*;
        match self {
            Acquire(lock) => write!(f, "acquire {}", lock),
            Release(lock) => write!(f, "release {}", lock),
        }
    }
}
