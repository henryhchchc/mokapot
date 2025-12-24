use std::collections::HashSet;

use super::super::Operand;
use crate::ir::Identifier;

/// An operation on a lock.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation {
    /// Acquires the lock.
    #[display("acquire {_0}")]
    Acquire(Operand),
    /// Releases the lock.
    #[display("release {_0}")]
    Release(Operand),
}

impl Operation {
    /// Returns the set of [`Identifier`]s used by the expression.
    #[must_use]
    pub fn uses(&self) -> HashSet<Identifier> {
        match self {
            Self::Acquire(arg) | Self::Release(arg) => arg.iter().copied().collect(),
        }
    }
}

#[cfg(test)]
mod tests {

    use proptest::prelude::*;

    use super::*;

    proptest! {

        #[test]
        fn uses(lock in any::<Operand>()) {
            let ids = lock.iter().copied().collect::<HashSet<_>>();
            let operation = Operation::Acquire(lock.clone());
            assert_eq!(operation.uses(), ids);

            let operation = Operation::Release(lock.clone());
            assert_eq!(operation.uses(), ids);
        }
    }
}
