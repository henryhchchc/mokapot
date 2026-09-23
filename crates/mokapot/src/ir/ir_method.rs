use super::{
    BasicBlock, BlockId, InstructionLocation, InstructionRef, MethodEntry, MokaIRBuildError,
    MokaIRMethod, SourceMap, ValueDefinition, ValueId,
};
use crate::{
    jvm::{Method, method, references::ClassRef},
    types::method_descriptor::MethodDescriptor,
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

    /// Returns the method access flags.
    #[must_use]
    pub const fn access_flags(&self) -> method::AccessFlags {
        self.access_flags
    }

    /// Returns the method name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the method descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &MethodDescriptor {
        &self.descriptor
    }

    /// Returns the class containing this method.
    #[must_use]
    pub const fn owner(&self) -> &ClassRef {
        &self.owner
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

    /// Returns the method-entry invocation.
    #[must_use]
    pub const fn entry(&self) -> &MethodEntry {
        &self.entry
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

    /// Returns this method's source-provenance relation.
    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Returns the SSA value representing `this`, if this is an instance method.
    #[must_use]
    pub const fn this_value(&self) -> Option<ValueId> {
        self.this
    }

    /// Returns the SSA values representing method parameters in descriptor order.
    #[must_use]
    pub fn parameter_values(&self) -> &[ValueId] {
        &self.parameters
    }

    /// Returns the unique definition of a method-local SSA value.
    ///
    /// Value identities are opaque and may be sparse. An identity with no
    /// retained definition yields `None`.
    #[must_use]
    pub fn definition_of(&self, value: ValueId) -> Option<ValueDefinition> {
        self.value_definitions.get(&value).copied()
    }
}
