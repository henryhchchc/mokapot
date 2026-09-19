use std::collections::{BTreeMap, HashMap};

use super::{
    BasicBlock, BlockId, BlockParameter, InstructionLocation, MokaIRBuildError, Operation,
    SourceMap, Terminator, ValueDefinition, ValueId, control_flow::ControlFlowGraph, generator,
};
use crate::{
    jvm::{self, Method as JvmMethod, method, references::ClassRef},
    types::method_descriptor::MethodDescriptor,
};

/// A completed scalar-SSA representation of the reachable part of a JVM method.
///
/// Blocks, edges, and values have opaque identities local to this method.
/// Value identities may be sparse and must not be interpreted as a count or
/// ordering of definitions.
/// Instructions are addressed by structural locations, and block terminators
/// are the sole source of control-flow edges.
#[derive(Debug, Clone)]
pub struct MokaIRMethod {
    access_flags: method::AccessFlags,
    name: String,
    descriptor: MethodDescriptor,
    owner: ClassRef,
    entry: MethodEntry,
    blocks: BTreeMap<BlockId, BasicBlock>,
    source_map: SourceMap,
    this_value: Option<ValueId>,
    parameter_values: Vec<ValueId>,
    value_definitions: HashMap<ValueId, ValueDefinition>,
}

/// A borrowed IR instruction resolved from an [`InstructionLocation`].
///
/// This enum preserves which kind of instruction was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionRef<'method> {
    /// A parameter bound on block entry.
    BlockParameter(&'method BlockParameter),
    /// An ordinary operation.
    Operation(&'method Operation),
    /// A block terminator.
    Terminator(&'method Terminator),
}

pub(crate) struct MokaIRMethodParts {
    pub(crate) entry: MethodEntry,
    pub(crate) blocks: BTreeMap<BlockId, BasicBlock>,
    pub(crate) source_map: SourceMap,
    pub(crate) this_value: Option<ValueId>,
    pub(crate) parameter_values: Vec<ValueId>,
    pub(crate) value_definitions: HashMap<ValueId, ValueDefinition>,
}

/// The invocation boundary that supplies arguments to the method's entry block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodEntry {
    pub(super) target: BlockId,
    pub(super) arguments: Vec<ValueId>,
}

impl MethodEntry {
    /// Returns the invoked entry block.
    #[must_use]
    pub const fn target(&self) -> BlockId {
        self.target
    }

    /// Returns the values supplied to the entry block's parameters.
    #[must_use]
    pub fn arguments(&self) -> &[ValueId] {
        &self.arguments
    }
}

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
        self.entry.target
    }

    /// Returns the method-entry invocation.
    #[must_use]
    pub const fn entry(&self) -> &MethodEntry {
        &self.entry
    }

    /// Returns all reachable blocks in deterministic identity order.
    ///
    /// Each block exposes entry parameters, ordered operations, and one terminator.
    #[must_use]
    pub fn blocks(&self) -> impl ExactSizeIterator<Item = (BlockId, &BasicBlock)> {
        self.blocks.iter().map(|(&id, block)| (id, block))
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
        self.this_value
    }

    /// Returns the SSA values representing method parameters in descriptor order.
    #[must_use]
    pub fn parameter_values(&self) -> &[ValueId] {
        &self.parameter_values
    }

    /// Returns the unique definition of a method-local SSA value.
    ///
    /// Value identities are opaque and may be sparse. An identity with no
    /// retained definition yields `None`.
    #[must_use]
    pub fn definition_of(&self, value: ValueId) -> Option<ValueDefinition> {
        self.value_definitions.get(&value).copied()
    }

    pub(crate) fn new(method: &jvm::Method, parts: MokaIRMethodParts) -> Self {
        Self {
            access_flags: method.access_flags,
            name: method.name.clone(),
            descriptor: method.descriptor.clone(),
            owner: method.owner.clone(),
            entry: parts.entry,
            blocks: parts.blocks,
            source_map: parts.source_map,
            this_value: parts.this_value,
            parameter_values: parts.parameter_values,
            value_definitions: parts.value_definitions,
        }
    }

    #[cfg(test)]
    pub(crate) const fn value_definitions(&self) -> &HashMap<ValueId, ValueDefinition> {
        &self.value_definitions
    }

    #[cfg(test)]
    pub(crate) const fn entry_mut(&mut self) -> &mut MethodEntry {
        &mut self.entry
    }

    #[cfg(test)]
    pub(crate) const fn blocks_mut(&mut self) -> &mut BTreeMap<BlockId, BasicBlock> {
        &mut self.blocks
    }

    /// Returns a borrowed control-flow view derived from block terminators.
    ///
    /// The returned view does not store an independent edge set.
    #[must_use]
    pub const fn control_flow_graph(&self) -> ControlFlowGraph<'_> {
        ControlFlowGraph::new(&self.blocks, self.entry.target)
    }
}
