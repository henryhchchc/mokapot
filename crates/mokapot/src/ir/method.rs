use std::collections::BTreeMap;

use super::{
    BasicBlock, BlockId, MokaIRBuildError, SourceMap, ValueDefinition, ValueId,
    control_flow::ControlFlowGraph, generator,
};
use crate::{
    jvm::{self, Method as JvmMethod, method, references::ClassRef},
    types::method_descriptor::MethodDescriptor,
};

/// A completed scalar-SSA representation of the reachable part of a JVM method.
///
/// Blocks, instructions, edges, and values have opaque identities local to this
/// method. Its block terminators are the sole source of control-flow edges.
#[derive(Debug, Clone)]
pub struct MokaIRMethod {
    access_flags: method::AccessFlags,
    name: String,
    descriptor: MethodDescriptor,
    owner: ClassRef,
    entry_block: BlockId,
    blocks: Vec<BasicBlock>,
    source_map: SourceMap,
    this_value: Option<ValueId>,
    parameter_values: Vec<ValueId>,
    caught_exceptions: BTreeMap<BlockId, ValueId>,
    value_definitions: Vec<ValueDefinition>,
}

impl MokaIRMethod {
    /// Builds completed `MokaIR` from a JVM method.
    ///
    /// JVM stack and local state are eliminated during construction. Legacy
    /// subroutines are expanded into context-specific control flow, trivial phis
    /// are removed, and only reachable blocks are emitted.
    ///
    /// # Errors
    ///
    /// Returns [`MokaIRBuildError`] when the method has no body, its reachable
    /// bytecode is invalid or unsupported, legacy-subroutine expansion exceeds
    /// its safety limit, or an internal construction invariant is violated.
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
    ///
    /// Each block exposes entry phis, ordered operations, and one terminator.
    /// Identities are dense and ascending here, so the block at index `i` has
    /// identity index `i`.
    #[must_use]
    pub fn blocks(&self) -> impl ExactSizeIterator<Item = &BasicBlock> {
        self.blocks.iter()
    }

    /// Looks up a block by its method-local identity.
    ///
    /// Resolution treats the identity as a position, sound only because
    /// [`MokaIRMethod::blocks`] stores identities densely and ascendingly; a
    /// sparse scheme would silently reject blocks. Identities outside the block
    /// range yield `None`.
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
    ///
    /// Handler-entry blocks and their caught values are synthetic and therefore
    /// need not have JVM source provenance.
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

    #[expect(clippy::too_many_arguments, reason = "TODO")]
    pub(crate) fn new(
        method: &jvm::Method,
        entry_block: BlockId,
        blocks: Vec<BasicBlock>,
        source_map: SourceMap,
        this_value: Option<ValueId>,
        parameter_values: Vec<ValueId>,
        caught_exceptions: BTreeMap<BlockId, ValueId>,
        value_definitions: Vec<ValueDefinition>,
    ) -> Self {
        Self {
            access_flags: method.access_flags,
            name: method.name.clone(),
            descriptor: method.descriptor.clone(),
            owner: method.owner.clone(),
            entry_block,
            blocks,
            source_map,
            this_value,
            parameter_values,
            caught_exceptions,
            value_definitions,
        }
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
    ///
    /// The returned view does not store an independent edge set.
    #[must_use]
    pub fn control_flow_graph(&self) -> ControlFlowGraph<'_> {
        ControlFlowGraph::new(&self.blocks, self.entry_block)
    }
}
