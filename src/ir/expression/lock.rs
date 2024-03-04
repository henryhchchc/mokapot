use std::collections::BTreeSet;
use std::fmt::Formatter;

use std::fmt::Display;

use crate::ir::Identifier;

use super::super::Argument;

/// An operation on a lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Acquires the lock.
    Acquire(Argument),
    /// Releases the lock.
    Release(Argument),
}
impl Operation {
    /// Returns the set of [`Identifier`]s used by the expression.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::Acquire(arg) | Self::Release(arg) => arg.iter().copied().collect(),
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Acquire(lock) => write!(f, "acquire {lock}"),
            Self::Release(lock) => write!(f, "release {lock}"),
        }
    }
}
