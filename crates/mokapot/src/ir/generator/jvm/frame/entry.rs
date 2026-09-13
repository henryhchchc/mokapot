#[derive(Debug, PartialEq, Eq, Clone, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Entry<V> {
    Value(V),
    #[display("<top>")]
    Top,
    #[display("<unset_local>")]
    UnsetLocal,
    #[display("<unavailable>")]
    Unavailable,
}

impl<V> Entry<V> {
    pub fn merge_from_with(
        &mut self,
        other: Self,
        join_values: impl FnOnce(&mut V, V) -> bool,
    ) -> bool {
        use Entry::{Top, Unavailable, UnsetLocal, Value};
        match (self, other) {
            (Value(lhs), Value(rhs)) => join_values(lhs, rhs),
            (Top, Top) | (UnsetLocal, UnsetLocal | Value(_)) | (Unavailable, _) => false,
            (slot @ Value(_), UnsetLocal) => {
                *slot = UnsetLocal;
                true
            }
            // Different slot shapes indicate local-variable slot reuse. Such a
            // slot is unavailable until a later instruction overwrites it.
            (slot, Top | Unavailable) | (slot @ Top, _) => {
                *slot = Unavailable;
                true
            }
        }
    }
}
