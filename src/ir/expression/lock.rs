use std::collections::BTreeSet;

use crate::ir::Identifier;

use super::super::Operand;

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
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::Acquire(arg) | Self::Release(arg) => arg.iter().copied().collect(),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::ir::test::arb_argument;

    use super::*;
    use proptest::prelude::*;

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
