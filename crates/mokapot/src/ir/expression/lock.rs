use std::collections::HashSet;

use crate::ir::ValueId;

/// An operation on a lock.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation<OP = ValueId> {
    /// Acquires the lock.
    #[display("acquire {_0}")]
    Acquire(OP),
    /// Releases the lock.
    #[display("release {_0}")]
    Release(OP),
}

impl Operation<ValueId> {
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

    use proptest::prelude::*;

    use super::*;

    proptest! {

        #[test]
        fn uses(lock in any::<ValueId>()) {
            let ids = HashSet::from([lock]);
            let operation = Operation::Acquire(lock);
            assert_eq!(operation.uses(), ids);

            let operation = Operation::Release(lock);
            assert_eq!(operation.uses(), ids);
        }
    }
}
