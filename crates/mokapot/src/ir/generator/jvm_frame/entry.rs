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
            (Entry::UninitializedLocal, _) | (_, Entry::OutOfScope) => Some(Less),
            (_, Entry::UninitializedLocal) | (Entry::OutOfScope, _) => Some(Greater),
        }
    }
}

impl<V: JoinSemiLattice> JoinSemiLattice for Entry<V> {
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
        fn join(mut self, other: Self) -> Self {
            self.0.extend(other.0);
            self
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
