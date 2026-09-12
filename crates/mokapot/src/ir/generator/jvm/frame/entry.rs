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
