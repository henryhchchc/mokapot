use std::collections::{BTreeMap, BTreeSet};

use super::InstructionId;
use crate::jvm::code::ProgramCounter;

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
mod tests;
