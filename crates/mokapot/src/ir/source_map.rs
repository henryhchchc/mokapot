use std::collections::{BTreeMap, BTreeSet};

use super::InstructionLocation;
use crate::jvm::code::ProgramCounter;

/// A sparse, bidirectional relation between JVM locations and `MokaIR` nodes.
///
/// This is not a bijection. A JVM instruction may have zero, one, or several
/// related IR nodes, and a synthetic IR node may have no JVM origin.
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    by_pc: BTreeMap<ProgramCounter, BTreeSet<InstructionLocation>>,
    by_location: BTreeMap<InstructionLocation, BTreeSet<ProgramCounter>>,
}

impl SourceMap {
    pub(crate) fn insert(&mut self, pc: ProgramCounter, instruction: InstructionLocation) {
        self.by_pc.entry(pc).or_default().insert(instruction);
        self.by_location.entry(instruction).or_default().insert(pc);
    }

    /// Returns every IR node directly related to a JVM instruction location.
    ///
    /// The iterator is empty when lifting erased the instruction without
    /// producing a semantic IR node. Resolve yielded locations with
    /// [`MokaIRMethod::instruction`](super::MokaIRMethod::instruction).
    pub fn instructions_at(
        &self,
        pc: ProgramCounter,
    ) -> impl Iterator<Item = InstructionLocation> + '_ {
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
        instruction: InstructionLocation,
    ) -> impl Iterator<Item = ProgramCounter> + '_ {
        self.by_location
            .get(&instruction)
            .into_iter()
            .flat_map(|locations| locations.iter().copied())
    }
}

#[cfg(test)]
mod tests;
