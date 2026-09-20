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
        assert!(
            self.by_location.insert(instruction, pc).is_none(),
            "an IR instruction location cannot have multiple JVM origins"
        );
        self.by_pc.entry(pc).or_default().push(instruction);
    }
}

#[cfg(test)]
impl SourceMap {
    /// Panics unless every recorded location resolves to a live instruction and
    /// the relation is bidirectional.
    pub(super) fn verify(&self, method: &super::MokaIRMethod) {
        for (&location, &pc) in &self.by_location {
            assert!(
                method.instruction(location).is_some(),
                "source location {pc} refers to missing instruction {location:?}"
            );
            assert!(
                self.instructions_at(pc)
                    .any(|candidate| candidate == location),
                "source mapping between {pc} and {location:?} is not bidirectional"
            );
        }
        for (&pc, locations) in &self.by_pc {
            for &location in locations {
                assert_eq!(
                    self.by_location.get(&location),
                    Some(&pc),
                    "source mapping between {pc} and {location:?} is not bidirectional"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BlockId, NumericalId};
    use std::collections::HashSet;

    #[test]
    fn source_map_is_sparse_and_one_to_many() {
        let pc0 = ProgramCounter::from(0);
        let pc1 = ProgramCounter::from(100);
        let pc2 = ProgramCounter::from(200);
        let instruction0 = InstructionLocation::Operation {
            block: BlockId::from_raw(0),
            index: 0,
        };
        let instruction1 = InstructionLocation::Operation {
            block: BlockId::from_raw(0),
            index: 1,
        };
        let instruction2 = InstructionLocation::Operation {
            block: BlockId::from_raw(1),
            index: 0,
        };
        let instruction3 = InstructionLocation::Terminator {
            block: BlockId::from_raw(1),
        };
        let synthetic = InstructionLocation::BlockParameter {
            block: BlockId::from_raw(1),
            index: 0,
        };
        let mut map = SourceMap::new();
        map.record_operation(pc0, BlockId::from_raw(0), 0);
        map.record_operation(pc0, BlockId::from_raw(0), 1);
        map.record_operation(pc1, BlockId::from_raw(1), 0);
        map.record_terminator(pc2, BlockId::from_raw(1));

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
