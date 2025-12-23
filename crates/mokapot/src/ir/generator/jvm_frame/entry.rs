use crate::ir::Operand;

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Entry {
    Value(Operand),
    #[display("<top>")]
    Top,
    #[display("<uninitialized_local>")]
    UninitializedLocal,
}

impl Entry {
    pub fn merge(lhs: Self, rhs: Self) -> Self {
        #[allow(clippy::enum_glob_use)]
        use Entry::*;
        match (lhs, rhs) {
            (Value(lhs), Value(rhs)) => Value(lhs | rhs),
            (Top, Top) => Top,
            (UninitializedLocal, it) | (it, UninitializedLocal) => it,
            // NOTE: When `lhs` and `rhs` are different variants, it indicates that the local
            //       variable slot is reused. In this case, we do not merge it since it will be
            //       overridden afterwards.
            (lhs, _) => lhs,
        }
    }
}
