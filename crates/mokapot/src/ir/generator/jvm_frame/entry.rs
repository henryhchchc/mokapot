use crate::analysis::fixed_point::JoinSemiLattice;

#[derive(Debug, PartialEq, Eq, Clone, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Entry<V> {
    Value(V),
    #[display("<top>")]
    Top,
    #[display("<uninitialized_local>")]
    UninitializedLocal,
    #[display("<out_of_scope>")]
    OutOfScope,
}

impl<V: PartialOrd> PartialOrd for Entry<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::{Equal, Greater, Less};
        match (self, other) {
            (Entry::Value(lhs), Entry::Value(rhs)) => lhs.partial_cmp(rhs),
            (Entry::Value(_), Entry::Top) | (Entry::Top, Entry::Value(_)) => None,
            (Entry::Top, Entry::Top)
            | (Entry::UninitializedLocal, Entry::UninitializedLocal)
            | (Entry::OutOfScope, Entry::OutOfScope) => Some(Equal),
            (Entry::Value(_), Entry::UninitializedLocal) | (_, Entry::OutOfScope) => Some(Less),
            (Entry::UninitializedLocal, Entry::Value(_)) | (Entry::OutOfScope, _) => Some(Greater),
            (Entry::Top, Entry::UninitializedLocal) | (Entry::UninitializedLocal, Entry::Top) => {
                None
            }
        }
    }
}

impl<V: JoinSemiLattice> JoinSemiLattice for Entry<V> {
    fn join_assign(&mut self, other: Self) -> bool {
        self.join_assign_with(
            other,
            crate::analysis::fixed_point::JoinSemiLattice::join_assign,
        )
    }
}

impl<V> Entry<V> {
    pub(super) fn join_assign_with(
        &mut self,
        other: Self,
        join_values: impl FnOnce(&mut V, V) -> bool,
    ) -> bool {
        use Entry::{OutOfScope, Top, UninitializedLocal, Value};
        match (self, other) {
            (Value(lhs), Value(rhs)) => join_values(lhs, rhs),
            (Top, Top) | (UninitializedLocal, UninitializedLocal | Value(_)) | (OutOfScope, _) => {
                false
            }
            (slot @ Value(_), UninitializedLocal) => {
                *slot = UninitializedLocal;
                true
            }
            // Different slot shapes indicate local-variable slot reuse. Such a
            // slot is unavailable until a later instruction overwrites it.
            (slot, Top | OutOfScope) | (slot @ Top, _) => {
                *slot = OutOfScope;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use proptest::prelude::*;

    #[derive(Debug, Clone, PartialEq, Eq, proptest_derive::Arbitrary)]
    struct TestSet(BTreeSet<u8>);

    impl PartialOrd for TestSet {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            if self == other {
                Some(std::cmp::Ordering::Equal)
            } else if self.0.is_subset(&other.0) {
                Some(std::cmp::Ordering::Less)
            } else if self.0.is_superset(&other.0) {
                Some(std::cmp::Ordering::Greater)
            } else {
                None
            }
        }
    }

    impl JoinSemiLattice for TestSet {
        fn join_assign(&mut self, other: Self) -> bool {
            let old_len = self.0.len();
            self.0.extend(other.0);
            self.0.len() != old_len
        }
    }

    proptest! {
       #[test]
       fn entry_join_ordering(
           lhs in any::<Entry<TestSet>>(),
           rhs in any::<Entry<TestSet>>()
       ) {
           let joined = lhs.clone().join(rhs.clone());
           prop_assert!(joined >= lhs);
           prop_assert!(joined >= rhs);
       }
    }
}
