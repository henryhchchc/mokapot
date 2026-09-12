/// A symbolic definition identity shared by frame analysis and SSA construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display("%ssa{_0}")]
pub(in crate::ir::generator) struct SsaValueId(u32);

impl SsaValueId {
    pub(in crate::ir::generator) const fn new(index: u32) -> Self {
        Self(index)
    }
}
