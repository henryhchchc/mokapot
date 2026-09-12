use std::collections::{BTreeMap, BTreeSet};

use super::InstructionId;
use crate::jvm::code::ProgramCounter;

/// A sparse, bidirectional relation between JVM locations and `MokaIR` nodes.
///
/// This is not a bijection. A JVM instruction may have zero, one, or several
/// related IR nodes, and a synthetic IR node may have no JVM origin.
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

    /// Returns every IR node directly related to a JVM instruction location.
    ///
    /// The iterator is empty when lifting erased the instruction without
    /// producing a semantic IR node.
    pub fn instructions_at(&self, pc: ProgramCounter) -> impl Iterator<Item = InstructionId> + '_ {
        self.by_pc
            .get(&pc)
            .into_iter()
            .flat_map(|nodes| nodes.iter().copied())
    }

    /// Returns every JVM instruction location directly related to an IR node.
    ///
    /// The iterator is empty for phis and other synthetic nodes. Consumers must
    /// not infer source coverage for such nodes.
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
