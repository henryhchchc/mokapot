use std::collections::HashSet;

use super::ValueId;

/// An operation on a lock.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation {
    /// Acquires the lock.
    #[display("acquire {_0}")]
    Acquire(ValueId),
    /// Releases the lock.
    #[display("release {_0}")]
    Release(ValueId),
}

impl Operation {
    /// Returns the values used by the expression.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::Acquire(arg) | Self::Release(arg) => HashSet::from([*arg]),
        }
    }
}
