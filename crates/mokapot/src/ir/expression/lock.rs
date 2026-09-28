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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test::prelude::ids;

    #[test]
    fn uses_reports_locked_value() {
        let [value] = ids(0);
        let acquire = Operation::Acquire(value);
        let release = Operation::Release(value);

        assert_eq!(acquire.uses(), HashSet::from([value]));
        assert_eq!(release.uses(), HashSet::from([value]));
    }
}
