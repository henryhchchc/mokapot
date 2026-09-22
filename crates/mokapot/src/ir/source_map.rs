use std::collections::HashMap;

use super::{BlockId, InstructionLocation};
use crate::jvm::code::ProgramCounter;

/// A sparse, bidirectional relation between JVM locations and `MokaIR` nodes.
///
/// This is not a bijection. A JVM instruction may have zero, one, or several
/// related IR nodes, while each non-synthetic IR node has at most one JVM
/// origin.
#[derive(Debug, Clone)]
pub struct SourceMap {
    by_pc: HashMap<ProgramCounter, Vec<InstructionLocation>>,
    by_location: HashMap<InstructionLocation, ProgramCounter>,
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
    pub(super) fn new() -> Self {
        Self {
            by_pc: HashMap::new(),
            by_location: HashMap::new(),
        }
    }

    pub(super) fn record_operation(&mut self, pc: ProgramCounter, block: BlockId, index: usize) {
        self.record(pc, InstructionLocation::Operation { block, index });
    }

    pub(super) fn record_terminator(&mut self, pc: ProgramCounter, block: BlockId) {
        self.record(pc, InstructionLocation::Terminator { block });
    }

    fn record(&mut self, pc: ProgramCounter, instruction: InstructionLocation) {
        let replaced = self.by_location.insert(instruction, pc);
        debug_assert!(
            replaced.is_none(),
            "an instruction location was recorded twice"
        );
        self.by_pc.entry(pc).or_default().push(instruction);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::ir::test::prelude::*;

    #[test]
    fn source_map_is_sparse_and_one_to_many() {
        let [b0, b1] = ids(0);
        let (pc0, pc1, pc2) = (0.into(), 100.into(), 200.into());
        let operation = |block, index| InstructionLocation::Operation { block, index };
        let terminator = |block| InstructionLocation::Terminator { block };
        let parameter = |block, index| InstructionLocation::BlockParameter { block, index };
        let instruction0 = operation(b0, 0);
        let instruction1 = operation(b0, 1);
        let instruction2 = operation(b1, 0);
        let instruction3 = terminator(b1);
        let synthetic = parameter(b1, 0);

        let mut map = SourceMap::new();
        map.record_operation(pc0, b0, 0);
        map.record_operation(pc0, b0, 1);
        map.record_operation(pc1, b1, 0);
        map.record_terminator(pc2, b1);

        assert_eq!(
            map.instructions_at(pc0).collect::<Vec<_>>(),
            [instruction0, instruction1]
        );
        assert_eq!(map.origin_of(instruction0), Some(pc0));
        assert_eq!(map.origin_of(instruction1), Some(pc0));
        assert_eq!(map.origin_of(instruction2), Some(pc1));
        assert_eq!(map.origin_of(instruction3), Some(pc2));
        assert_eq!(map.instructions_at(pc2).collect::<Vec<_>>(), [instruction3]);
        assert_eq!(map.instructions_at(50.into()).count(), 0);
        assert_eq!(map.origin_of(synthetic), None);

        let covered_nodes = HashSet::from([pc0])
            .into_iter()
            .flat_map(|pc| map.instructions_at(pc))
            .collect::<HashSet<_>>();
        assert_eq!(covered_nodes, HashSet::from([instruction0, instruction1]));
        assert!(!covered_nodes.contains(&instruction2));
        assert!(!covered_nodes.contains(&synthetic));
    }
}
