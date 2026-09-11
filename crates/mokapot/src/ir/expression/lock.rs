use std::collections::HashSet;

use crate::ir::{TryMapValues, ValueId};

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

impl<OP, OUT> TryMapValues<OUT> for Operation<OP> {
    type Value = OP;
    type Mapped = Operation<OUT>;

    fn try_map_values<E>(
        self,
        mut remap: impl FnMut(OP) -> Result<OUT, E>,
    ) -> Result<Operation<OUT>, E> {
        Ok(match self {
            Self::Acquire(value) => Operation::Acquire(remap(value)?),
            Self::Release(value) => Operation::Release(remap(value)?),
        })
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
