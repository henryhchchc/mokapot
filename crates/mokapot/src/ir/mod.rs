//! `MokaIR` is an intermediate representation of JVM bytecode.
//! It is register based and is in SSA form, which make it easier to analyze.

pub mod control_flow;
pub mod data_flow;
pub mod expression;
mod generator;
mod moka_instruction;
#[cfg(feature = "petgraph")]
pub mod petgraph;

use std::collections::{BTreeMap, BTreeSet};

pub use data_flow::{DefUseChain, UseSite};
pub use generator::{MokaIRBrewingError, MokaIRMethodExt};
pub use moka_instruction::*;

use self::control_flow::ControlFlowGraph;
use crate::{
    jvm::{
        code::ProgramCounter,
        method::{self},
        references::ClassRef,
    },
    types::method_descriptor::MethodDescriptor,
};

/// Represents a JVM method where the instructions have been converted to Moka IR.
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
    #[expect(
        clippy::too_many_arguments,
        reason = "the private constructor assembles independently owned method metadata and IR"
    )]
    pub(crate) const fn new(
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
    ) -> Self {
        Self {
            access_flags,
            name,
            descriptor,
            owner,
            entry_block,
            blocks,
            source_map,
            this_value,
            parameter_values,
            caught_exceptions,
            value_definitions,
        }
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
    pub fn value_definition(&self, value: ValueId) -> Option<ValueDefinition> {
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

/// A sparse, bidirectional relation between JVM locations and Moka IR nodes.
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    by_pc: BTreeMap<ProgramCounter, BTreeSet<InstructionId>>,
    by_instruction: BTreeMap<InstructionId, BTreeSet<ProgramCounter>>,
}

impl SourceMap {
    pub(crate) fn insert(&mut self, pc: ProgramCounter, instruction: InstructionId) {
        self.by_pc.entry(pc).or_default().insert(instruction);
        self.by_instruction
            .entry(instruction)
            .or_default()
            .insert(pc);
    }

    /// Returns the IR nodes related to a JVM instruction location.
    pub fn instructions_at(&self, pc: ProgramCounter) -> impl Iterator<Item = InstructionId> + '_ {
        self.by_pc
            .get(&pc)
            .into_iter()
            .flat_map(|nodes| nodes.iter().copied())
    }

    /// Returns the JVM instruction locations related to an IR node.
    pub fn origins_of(
        &self,
        instruction: InstructionId,
    ) -> impl Iterator<Item = ProgramCounter> + '_ {
        self.by_instruction
            .get(&instruction)
            .into_iter()
            .flat_map(|locations| locations.iter().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_map_is_a_sparse_many_to_many_relation() {
        let pc0 = ProgramCounter::from(0);
        let pc1 = ProgramCounter::from(100);
        let pc2 = ProgramCounter::from(200);
        let instruction0 = InstructionId::new(0);
        let instruction1 = InstructionId::new(1);
        let instruction2 = InstructionId::new(2);
        let instruction3 = InstructionId::new(3);
        let synthetic = InstructionId::new(4);
        let mut map = SourceMap::default();
        map.insert(pc0, instruction0);
        map.insert(pc0, instruction1);
        map.insert(pc1, instruction1);
        map.insert(pc1, instruction2);
        map.insert(pc2, instruction3);

        assert_eq!(
            map.instructions_at(pc0).collect::<Vec<_>>(),
            [instruction0, instruction1]
        );
        assert_eq!(map.origins_of(instruction1).collect::<Vec<_>>(), [pc0, pc1]);
        assert_eq!(map.instructions_at(pc2).collect::<Vec<_>>(), [instruction3]);
        assert_eq!(map.instructions_at(50.into()).count(), 0);
        assert_eq!(map.origins_of(synthetic).count(), 0);

        let covered_nodes = BTreeSet::from([pc0])
            .into_iter()
            .flat_map(|pc| map.instructions_at(pc))
            .collect::<BTreeSet<_>>();
        assert_eq!(covered_nodes, BTreeSet::from([instruction0, instruction1]));
        assert!(!covered_nodes.contains(&instruction2));
        assert!(!covered_nodes.contains(&synthetic));
    }
}
