use crate::{
    ir::generator::bytecode_analysis::ScalarBlock,
    ir::{BlockId, ValueId},
};

/// A scalar SSA block ready for final IR emission.
pub(crate) struct Block {
    pub phis: Vec<Phi>,
    pub scalar: ScalarBlock,
}

/// A retained SSA phi and its predecessor-indexed inputs.
pub(crate) struct Phi {
    pub value: ValueId,
    pub inputs: Vec<(BlockId, ValueId)>,
}
