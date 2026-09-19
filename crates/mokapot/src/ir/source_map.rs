use std::collections::{BTreeMap, BTreeSet};

use super::InstructionLocation;
use crate::jvm::code::ProgramCounter;

/// A sparse, bidirectional relation between JVM locations and `MokaIR` nodes.
///
/// This is not a bijection. A JVM instruction may have zero, one, or several
/// related IR nodes, while each non-synthetic IR node has at most one JVM
/// origin.
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    by_pc: BTreeMap<ProgramCounter, BTreeSet<InstructionLocation>>,
    by_location: BTreeMap<InstructionLocation, ProgramCounter>,
}

impl SourceMap {
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

    /// Returns the JVM instruction location directly related to an IR node.
    ///
    /// Synthetic nodes, including block parameters, have no JVM origin.
    #[must_use]
    pub fn origin_of(&self, instruction: InstructionLocation) -> Option<ProgramCounter> {
        self.by_location.get(&instruction).copied()
    }
}

impl SourceMap {
    pub(super) fn record_operation(
        &mut self,
        pc: ProgramCounter,
        block: super::BlockId,
        index: usize,
    ) {
        self.record(pc, InstructionLocation::Operation { block, index });
    }

    pub(super) fn record_terminator(&mut self, pc: ProgramCounter, block: super::BlockId) {
        self.record(pc, InstructionLocation::Terminator { block });
    }

    fn record(&mut self, pc: ProgramCounter, instruction: InstructionLocation) {
        assert!(
            self.by_location.insert(instruction, pc).is_none(),
            "an IR instruction location cannot have multiple JVM origins"
        );
        self.by_pc.entry(pc).or_default().insert(instruction);
    }
}

#[cfg(test)]
mod tests;
