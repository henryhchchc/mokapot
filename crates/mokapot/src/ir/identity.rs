/// The identity of a basic block within one Moka IR method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
#[display("b{_0}")]
pub struct BlockId(u32);

impl BlockId {
    pub(super) const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// The structural location of an instruction within a method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionLocation {
    /// A block parameter at the given block-entry index.
    BlockParameter {
        /// The block containing the parameter.
        block: BlockId,
        /// The parameter's index in the block-entry list.
        index: usize,
    },
    /// An ordinary operation at the given block-local index.
    Operation {
        /// The block containing the operation.
        block: BlockId,
        /// The operation's index in the block's operation list.
        index: usize,
    },
    /// The terminator of the given block.
    Terminator {
        /// The block whose terminator is addressed.
        block: BlockId,
    },
}

/// The identity of a control-flow edge within one Moka IR method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
#[display("e{_0}")]
pub struct EdgeId(u32);

impl EdgeId {
    pub(super) const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// The opaque identity of a scalar value within one Moka IR method.
///
/// Identities may be sparse and convey neither definition order nor a value
/// count.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(transparent)]
#[display("%{_0}")]
pub struct ValueId(u32);

impl ValueId {
    pub(super) const fn new(index: u32) -> Self {
        Self(index)
    }
}
