//! `MokaIR` is an intermediate representation of JVM bytecode.
//! It is register based and is in SSA form, which make it easier to analyze.

pub mod control_flow;
pub mod data_flow;
pub mod expression;
mod generator;
mod moka_instruction;
#[cfg(feature = "petgraph")]
pub mod petgraph;

use std::collections::{BTreeMap, BTreeSet, HashMap};

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
}

impl MokaIRMethod {
    pub(crate) const fn new(
        access_flags: method::AccessFlags,
        name: String,
        descriptor: MethodDescriptor,
        owner: ClassRef,
        entry_block: BlockId,
        blocks: Vec<BasicBlock>,
        source_map: SourceMap,
    ) -> Self {
        Self {
            access_flags,
            name,
            descriptor,
            owner,
            entry_block,
            blocks,
            source_map,
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

    /// Returns a borrowed control-flow view derived from block terminators.
    #[must_use]
    pub fn control_flow_graph(&self) -> ControlFlowGraph<'_> {
        ControlFlowGraph::new(&self.blocks, self.entry_block)
    }

    #[cfg(feature = "petgraph")]
    fn instruction(&self, id: InstructionId) -> Option<&MokaInstruction> {
        self.blocks
            .iter()
            .flat_map(BasicBlock::instructions)
            .find(|instruction| instruction.id() == id)
    }

    #[cfg(feature = "petgraph")]
    pub(crate) fn uses_at(
        &self,
        id: InstructionId,
    ) -> Option<std::collections::HashSet<Identifier>> {
        self.instruction(id).map(MokaInstruction::uses).or_else(|| {
            self.blocks
                .iter()
                .map(BasicBlock::terminator)
                .find(|terminator| terminator.id() == id)
                .map(Terminator::uses)
        })
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

/// A def-use chain in data flow analysis.
#[derive(Debug)]
pub struct DefUseChain<'a> {
    #[cfg_attr(
        all(not(feature = "petgraph"), feature = "unstable-moka-ir"),
        expect(dead_code)
    )]
    method: &'a MokaIRMethod,
    defs: HashMap<ValueId, InstructionId>,
    uses: HashMap<Identifier, BTreeSet<InstructionId>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_map_is_a_sparse_many_to_many_relation() {
        let pc0 = ProgramCounter::from(0);
        let pc1 = ProgramCounter::from(100);
        let instruction0 = InstructionId::new(0);
        let instruction1 = InstructionId::new(1);
        let mut map = SourceMap::default();
        map.insert(pc0, instruction0);
        map.insert(pc0, instruction1);
        map.insert(pc1, instruction1);

        assert_eq!(
            map.instructions_at(pc0).collect::<Vec<_>>(),
            [instruction0, instruction1]
        );
        assert_eq!(map.origins_of(instruction1).collect::<Vec<_>>(), [pc0, pc1]);
        assert_eq!(map.instructions_at(50.into()).count(), 0);
        assert_eq!(map.origins_of(InstructionId::new(50)).count(), 0);
    }
}
