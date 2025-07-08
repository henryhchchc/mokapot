use std::collections::BTreeSet;

use super::super::Operand;
use crate::ir::Identifier;

/// An operation on a lock.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
#[instability::unstable(feature = "moka-ir")]
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
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::Acquire(arg) | Self::Release(arg) => arg.iter().copied().collect(),
        }
    }
}

#[cfg(test)]
mod tests {

    use proptest::prelude::*;

    use super::*;
    use crate::ir::test::arb_argument;

    proptest! {

        #[test]
        fn uses(lock in arb_argument()) {
            let ids = lock.iter().copied().collect::<BTreeSet<_>>();
            let operation = Operation::Acquire(lock.clone());
            assert_eq!(operation.uses(), ids);

            let operation = Operation::Release(lock.clone());
            assert_eq!(operation.uses(), ids);
        }
    }
}
