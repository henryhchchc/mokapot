use super::{
    BasicBlock, BlockId, InstructionId, MokaIRBuildError, Operation, Phi, SourceMap, Terminator,
    ValueDefinition, ValueId, control_flow::ControlFlowGraph, generator,
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
    value_definitions: Vec<ValueDefinition>,
    instruction_locations: Vec<InstructionLocation>,
}

/// A borrowed IR instruction resolved from an [`InstructionId`].
///
/// Phis, ordinary operations, and terminators share one method-local identity
/// space, so this enum preserves which kind of instruction was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionRef<'method> {
    /// A phi evaluated on block entry.
    Phi(&'method Phi),
    /// An ordinary operation.
    Operation(&'method Operation),
    /// A block terminator.
    Terminator(&'method Terminator),
}

impl InstructionRef<'_> {
    /// Returns this instruction's method-local identity.
    #[must_use]
    pub const fn id(self) -> InstructionId {
        match self {
            Self::Phi(phi) => phi.id,
            Self::Operation(operation) => operation.id(),
            Self::Terminator(terminator) => terminator.id(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum InstructionLocation {
    Phi { block: BlockId, index: usize },
    Operation { block: BlockId, index: usize },
    Terminator { block: BlockId },
}

pub(crate) struct MokaIRMethodParts {
    pub(crate) entry_block: BlockId,
    pub(crate) blocks: Vec<BasicBlock>,
    pub(crate) source_map: SourceMap,
    pub(crate) this_value: Option<ValueId>,
    pub(crate) parameter_values: Vec<ValueId>,
    pub(crate) value_definitions: Vec<ValueDefinition>,
    pub(crate) instruction_locations: Vec<InstructionLocation>,
}

impl MokaIRMethod {
    /// Builds completed `MokaIR` from a JVM method.
    ///
    /// JVM stack and local state are eliminated during construction, trivial
    /// phis are removed, and only reachable blocks are emitted.
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

    /// Resolves an instruction, phi, or terminator by its method-local identity.
    ///
    /// Identities outside this method's dense instruction space yield `None`.
    #[must_use]
    pub fn instruction(&self, id: InstructionId) -> Option<InstructionRef<'_>> {
        let location = self
            .instruction_locations
            .get(usize::try_from(id.index()).ok()?)?;
        Some(match *location {
            InstructionLocation::Phi { block, index } => {
                InstructionRef::Phi(self.block(block)?.phis.get(index)?)
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

    /// Returns the caught-exception value introduced by a synthetic handler-entry block.
    ///
    /// Handler-entry blocks and their caught values are synthetic and therefore
    /// need not have JVM source provenance.
    #[must_use]
    pub fn caught_exception(&self, block: BlockId) -> Option<ValueId> {
        self.block(block)?.caught_exception
    }

    /// Returns the unique definition of a method-local SSA value.
    #[must_use]
    pub fn definition_of(&self, value: ValueId) -> Option<ValueDefinition> {
        self.value_definitions
            .get(usize::try_from(value.index()).ok()?)
            .copied()
    }

    pub(crate) fn new(method: &jvm::Method, parts: MokaIRMethodParts) -> Self {
        Self {
            access_flags: method.access_flags,
            name: method.name.clone(),
            descriptor: method.descriptor.clone(),
            owner: method.owner.clone(),
            entry_block: parts.entry_block,
            blocks: parts.blocks,
            source_map: parts.source_map,
            this_value: parts.this_value,
            parameter_values: parts.parameter_values,
            value_definitions: parts.value_definitions,
            instruction_locations: parts.instruction_locations,
        }
    }

    /// Returns a borrowed control-flow view derived from block terminators.
    ///
    /// The returned view does not store an independent edge set.
    #[must_use]
    pub fn control_flow_graph(&self) -> ControlFlowGraph<'_> {
        ControlFlowGraph::new(&self.blocks, self.entry_block)
    }
}
