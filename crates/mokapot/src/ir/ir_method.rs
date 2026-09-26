use super::{
    BasicBlock, BlockId, InstructionLocation, InstructionRef, MokaIRMethod, ValueDefinition,
    ValueId,
};
use crate::{
    ir::MokaIRBuildError,
    jvm::{Method, method},
};

impl MokaIRMethod {
    /// Builds completed `MokaIR` from a JVM method.
    ///
    /// JVM stack and local state are eliminated during construction, trivial
    /// block parameters are simplified, and only reachable blocks are emitted.
    ///
    /// # Errors
    ///
    /// Returns [`MokaIRBuildError`] when the method has no body, uses unsupported
    /// bytecode, has invalid bytecode structure or reachable frame state, or an
    /// internal construction invariant is violated.
    pub fn from_method(method: &Method) -> Result<Self, MokaIRBuildError> {
        super::generator::generate(method)
    }

    /// Checks if the method is `static`.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.access_flags.contains(method::AccessFlags::STATIC)
    }

    /// Returns the entry block identity.
    #[must_use]
    pub const fn entry_block(&self) -> BlockId {
        self.entry.target
    }

    /// Looks up a block by its method-local identity.
    ///
    /// Identities outside this method's block set yield `None`.
    #[must_use]
    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(&id)
    }

    /// Resolves a block parameter, operation, or terminator by structural location.
    ///
    /// Locations outside this method's block structure yield `None`.
    #[must_use]
    pub fn instruction(&self, location: InstructionLocation) -> Option<InstructionRef<'_>> {
        Some(match location {
            InstructionLocation::BlockParameter { block, index } => {
                InstructionRef::BlockParameter(self.block(block)?.parameters.get(index)?)
            }
            InstructionLocation::Operation { block, index } => {
                InstructionRef::Operation(self.block(block)?.operations.get(index)?)
            }
            InstructionLocation::Terminator { block } => {
                InstructionRef::Terminator(&self.block(block)?.terminator)
            }
        })
    }

    /// Returns the unique definition of a method-local SSA value.
    ///
    /// An identity with no retained definition yields `None`.
    #[must_use]
    pub fn definition_of(&self, value: ValueId) -> Option<ValueDefinition> {
        self.value_definitions.get(&value).copied()
    }
}
