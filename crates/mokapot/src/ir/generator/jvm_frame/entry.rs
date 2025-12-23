use crate::{analysis::fixed_point::JoinSemiLattice, ir::Operand};

#[derive(Debug, PartialEq, Eq, Clone, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Entry {
    Value(Operand),
    #[display("<top>")]
    Top,
    #[display("<uninitialized_local>")]
    UninitializedLocal,
    #[display("<out_of_scope>")]
    OutOfScope,
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::{Equal, Greater, Less};
        match (self, other) {
            (Entry::Value(lhs), Entry::Value(rhs)) => lhs.partial_cmp(rhs),
            (Entry::Value(_), Entry::Top) | (Entry::Top, Entry::Value(_)) => None,
            (Entry::Top, Entry::Top)
            | (Entry::UninitializedLocal, Entry::UninitializedLocal)
            | (Entry::OutOfScope, Entry::OutOfScope) => Some(Equal),
            (Entry::UninitializedLocal, _) | (_, Entry::OutOfScope) => Some(Less),
            (_, Entry::UninitializedLocal) | (Entry::OutOfScope, _) => Some(Greater),
        }
    }
}

impl JoinSemiLattice for Entry {
    fn join(self, other: Self) -> Self {
        use Entry::{OutOfScope, Top, UninitializedLocal, Value};
        match (self, other) {
            (Value(lhs), Value(rhs)) => Value(lhs.join(rhs)),
            (Top, Top) => Top,
            (UninitializedLocal, it) | (it, UninitializedLocal) => it,
            // NOTE: When `lhs` and `rhs` are different variants, it indicates that the local
            //       variable slot is reused. In this case, we do not merge it since it will be
            //       overridden afterwards.
            (_, Top | OutOfScope) | (OutOfScope | Top, _) => OutOfScope,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
       #[test]
       fn entry_join_ordering(
           lhs in any::<Entry>(),
           rhs in any::<Entry>()
       ) {
           let joined = lhs.clone().join(rhs.clone());
           prop_assert!(joined >= lhs);
           prop_assert!(joined >= rhs);
       }
    }
}
