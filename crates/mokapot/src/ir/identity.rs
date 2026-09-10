/// The identity of a basic block within one Moka IR method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
#[display("b{_0}")]
pub struct BlockId(u32);

impl BlockId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
    pub(crate) const fn index(self) -> u32 {
        self.0
    }
}

/// The identity of an instruction, phi, or terminator within one Moka IR method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
#[display("i{_0}")]
pub struct InstructionId(u32);

impl InstructionId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// The identity of a control-flow edge within one Moka IR method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
#[display("e{_0}")]
pub struct EdgeId(u32);

impl EdgeId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// The identity of a scalar value within one Moka IR method.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(transparent)]
#[display("%{_0}")]
pub struct ValueId(u32);

impl ValueId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> u32 {
        self.0
    }
}
