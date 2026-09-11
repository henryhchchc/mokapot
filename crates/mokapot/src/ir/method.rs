use std::collections::BTreeMap;

use super::{
    BasicBlock, BlockId, MokaIRBuildError, SourceMap, ValueDefinition, ValueId,
    control_flow::ControlFlowGraph, generator,
};
use crate::{
    jvm::{Method as JvmMethod, method, references::ClassRef},
    types::method_descriptor::MethodDescriptor,
};

/// Represents a JVM method where the instructions have been converted to Moka IR.
#[derive(Debug, Clone)]
pub struct MokaIRMethod {
    pub(super) access_flags: method::AccessFlags,
    pub(super) name: String,
    pub(super) descriptor: MethodDescriptor,
    pub(super) owner: ClassRef,
    pub(super) entry_block: BlockId,
    pub(super) blocks: Vec<BasicBlock>,
    pub(super) source_map: SourceMap,
    pub(super) this_value: Option<ValueId>,
    pub(super) parameter_values: Vec<ValueId>,
    pub(super) caught_exceptions: BTreeMap<BlockId, ValueId>,
    pub(super) value_definitions: Vec<ValueDefinition>,
}

impl MokaIRMethod {
    /// Builds Moka IR from a JVM method.
    ///
    /// # Errors
    ///
    /// Returns [`MokaIRBuildError`] when the method has no body or its reachable
    /// control flow cannot be represented as valid Moka IR.
    pub fn from_method(method: &JvmMethod) -> Result<Self, MokaIRBuildError> {
        generator::generate(method)
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
        self.entry_block
    }

    /// Returns all reachable blocks in deterministic source order.
    #[must_use]
    pub fn blocks(&self) -> impl ExactSizeIterator<Item = &BasicBlock> {
        self.blocks.iter()
    }

    /// Looks up a block by its method-local identity.
    #[must_use]
    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(usize::try_from(id.index()).ok()?)
    }

    /// Returns this method's source-provenance relation.
    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Returns the SSA value representing `this`, if this is an instance method.
    #[must_use]
    pub const fn this_value(&self) -> Option<ValueId> {
        self.this_value
    }

    /// Returns the SSA values representing method parameters in descriptor order.
    #[must_use]
    pub fn parameter_values(&self) -> &[ValueId] {
        &self.parameter_values
    }

    /// Returns the caught-exception value introduced by a synthetic handler-entry block.
    #[must_use]
    pub fn caught_exception(&self, block: BlockId) -> Option<ValueId> {
        self.caught_exceptions.get(&block).copied()
    }

    /// Returns the unique definition of a method-local SSA value.
    #[must_use]
    pub fn definition_of(&self, value: ValueId) -> Option<ValueDefinition> {
        self.value_definitions
            .get(usize::try_from(value.index()).ok()?)
            .copied()
    }

    pub(crate) fn value_definitions(
        &self,
    ) -> impl Iterator<Item = (ValueId, ValueDefinition)> + '_ {
        self.value_definitions
            .iter()
            .copied()
            .enumerate()
            .map(|(id, definition)| {
                (
                    ValueId::new(u32::try_from(id).expect("value identity must fit u32")),
                    definition,
                )
            })
    }

    /// Returns a borrowed control-flow view derived from block terminators.
    #[must_use]
    pub fn control_flow_graph(&self) -> ControlFlowGraph<'_> {
        ControlFlowGraph::new(&self.blocks, self.entry_block)
    }
}
