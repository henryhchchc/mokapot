//! Test-only validation of completed Moka IR invariants.
//!
//! Verification lives beside the state it inspects so it needs no widened
//! visibility. Checks assert and thereby fail loudly on the first violated
//! invariant.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::ir::{BlockId, BlockKind, InstructionLocation, Terminator, ValueDefinition, ValueId};

use super::MokaIRMethod;

/// Where a value is read.
#[derive(Debug, Clone, Copy)]
enum UseSite {
    Operation {
        block: BlockId,
        index: usize,
    },
    Terminator {
        block: BlockId,
    },
    /// An argument supplied on an outgoing arm of `source`, tagged with whether
    /// that arm is the defining terminator's normal arm.
    EdgeArgument {
        source: BlockId,
        normal: bool,
    },
}

impl UseSite {
    const fn block(self) -> BlockId {
        match self {
            Self::Operation { block, .. } | Self::Terminator { block } => block,
            Self::EdgeArgument { source, .. } => source,
        }
    }
}

/// Returns the block that structurally contains `location`.
const fn location_block(location: InstructionLocation) -> BlockId {
    match location {
        InstructionLocation::BlockParameter { block, .. }
        | InstructionLocation::Operation { block, .. }
        | InstructionLocation::Terminator { block } => block,
    }
}

impl MokaIRMethod {
    /// Panics if this method violates an invariant of completed IR.
    pub(in crate::ir) fn verify(&self) {
        let blocks = self.collect_blocks();
        let predecessors = self.collect_predecessors(&blocks);
        self.verify_reachability(&blocks);
        let dominators = self.compute_dominators(&blocks, &predecessors);

        let definitions = self.collect_definitions();
        self.verify_definition_index(&definitions);
        self.verify_uses(&definitions, &dominators);
        self.source_map.verify(self);
    }

    fn collect_blocks(&self) -> HashSet<BlockId> {
        let entry = self.entry.target;
        assert!(
            self.blocks.contains_key(&entry),
            "entry block {entry} has no definition"
        );
        assert_eq!(
            self.entry.arguments.len(),
            self.blocks[&entry].parameters.len(),
            "method-entry argument count differs from entry parameter count"
        );
        self.blocks.keys().copied().collect()
    }

    /// Collects predecessors while checking that every edge matches its target.
    fn collect_predecessors(
        &self,
        blocks: &HashSet<BlockId>,
    ) -> HashMap<BlockId, HashSet<BlockId>> {
        let mut predecessors = blocks
            .iter()
            .map(|&block| (block, HashSet::new()))
            .collect::<HashMap<_, _>>();
        for (&source, block) in &self.blocks {
            for successor in block.terminator.successors() {
                let Some(target) = successor.block_target() else {
                    continue;
                };
                let Some(target_block) = self.blocks.get(&target) else {
                    panic!("the successor of block {source} targets undefined block {target}");
                };
                let expected = target_block.parameters.len();
                assert_eq!(
                    successor.arguments().len(),
                    expected,
                    "the edge {source} -> {target} supplies {} arguments to {expected} parameters",
                    successor.arguments().len(),
                );
                predecessors.entry(target).or_default().insert(source);
            }
        }
        predecessors
    }

    fn verify_reachability(&self, blocks: &HashSet<BlockId>) {
        let reachable = self.reachable_blocks(&HashSet::new());
        assert_eq!(
            &reachable,
            blocks,
            "completed IR contains unreachable blocks: {:?}",
            blocks.difference(&reachable).collect::<Vec<_>>()
        );
    }

    /// Returns the blocks reachable from the entry, skipping the normal arm of
    /// the terminators in `removed_normal`.
    fn reachable_blocks(&self, removed_normal: &HashSet<BlockId>) -> HashSet<BlockId> {
        let entry = self.entry.target;
        let mut reachable = HashSet::from([entry]);
        let mut pending = VecDeque::from([entry]);
        while let Some(block_id) = pending.pop_front() {
            let block = self
                .blocks
                .get(&block_id)
                .expect("the verifier only enqueues defined blocks");
            let removed = removed_normal.contains(&block_id);
            for successor in block.terminator.successors() {
                if removed
                    && let Terminator::Try { normal, .. } = &block.terminator
                    && std::ptr::eq(normal, successor)
                {
                    continue;
                }
                let Some(target) = successor.block_target() else {
                    continue;
                };
                if reachable.insert(target) {
                    pending.push_back(target);
                }
            }
        }
        reachable
    }

    fn compute_dominators(
        &self,
        blocks: &HashSet<BlockId>,
        predecessors: &HashMap<BlockId, HashSet<BlockId>>,
    ) -> HashMap<BlockId, HashSet<BlockId>> {
        let entry = self.entry.target;
        let mut dominators = blocks
            .iter()
            .map(|&block| {
                let initial = if block == entry {
                    HashSet::from([entry])
                } else {
                    blocks.clone()
                };
                (block, initial)
            })
            .collect::<HashMap<_, _>>();

        loop {
            let mut changed = false;
            for &block in blocks.iter().filter(|&&block| block != entry) {
                let mut incoming = predecessors[&block].iter();
                let Some(&first) = incoming.next() else {
                    panic!("non-entry block {block} has no predecessor");
                };
                let mut intersection = dominators[&first].clone();
                for predecessor in incoming {
                    intersection.retain(|candidate| dominators[predecessor].contains(candidate));
                }
                intersection.insert(block);
                if dominators[&block] != intersection {
                    dominators.insert(block, intersection);
                    changed = true;
                }
            }
            if !changed {
                return dominators;
            }
        }
    }

    /// Collects every defined value.
    fn collect_definitions(&self) -> HashMap<ValueId, ValueDefinition> {
        let mut definitions = HashMap::new();
        let mut define = |value: ValueId, definition: ValueDefinition| {
            let previous = definitions.insert(value, definition);
            assert!(
                previous.is_none(),
                "value {value} has multiple definitions: {previous:?} and {definition:?}"
            );
        };

        if let Some(value) = self.this_value {
            define(value, ValueDefinition::This);
        }
        for (index, &value) in self.parameter_values.iter().enumerate() {
            let index = u16::try_from(index).expect("method parameter index cannot be represented");
            define(value, ValueDefinition::Parameter(index));
        }
        for (&block_id, block) in &self.blocks {
            if let BlockKind::LandingPad { exception: value } = block.kind {
                define(value, ValueDefinition::CaughtException(block_id));
            }
            for (index, parameter) in block.parameters.iter().enumerate() {
                define(
                    parameter.value,
                    ValueDefinition::Instruction(InstructionLocation::BlockParameter {
                        block: block_id,
                        index,
                    }),
                );
            }
            for (index, operation) in block.operations.iter().enumerate() {
                let Some(value) = operation.def() else {
                    continue;
                };
                define(
                    value,
                    ValueDefinition::Instruction(InstructionLocation::Operation {
                        block: block_id,
                        index,
                    }),
                );
            }
            if let Terminator::Try { operation, .. } = &block.terminator
                && let Some(value) = operation.def()
            {
                define(
                    value,
                    ValueDefinition::Instruction(InstructionLocation::Terminator {
                        block: block_id,
                    }),
                );
            }
        }
        definitions
    }

    fn verify_definition_index(&self, definitions: &HashMap<ValueId, ValueDefinition>) {
        for (&value, &indexed) in &self.value_definitions {
            assert_eq!(
                definitions.get(&value).copied(),
                Some(indexed),
                "definition index disagrees for {value}"
            );
            if let ValueDefinition::Instruction(location) = indexed {
                assert!(
                    self.instruction(location).is_some(),
                    "definition of {value} refers to missing instruction {location:?}"
                );
            }
        }
        for (&value, &definition) in definitions {
            assert_eq!(
                self.definition_of(value),
                Some(definition),
                "definition index is missing live value {value} defined as {definition:?}"
            );
        }
    }

    fn verify_uses(
        &self,
        definitions: &HashMap<ValueId, ValueDefinition>,
        dominators: &HashMap<BlockId, HashSet<BlockId>>,
    ) {
        for &value in &self.entry.arguments {
            assert!(
                matches!(
                    definitions.get(&value),
                    Some(ValueDefinition::This | ValueDefinition::Parameter(_))
                ),
                "method-entry argument {value} is not an externally defined value"
            );
        }
        for (&block_id, block) in &self.blocks {
            let normal_arm = match &block.terminator {
                Terminator::Try { normal, .. } => Some(normal),
                _ => None,
            };
            for successor in block.terminator.successors() {
                let normal = normal_arm.is_some_and(|normal| std::ptr::eq(normal, successor));
                for &value in successor.arguments() {
                    self.verify_use(
                        value,
                        UseSite::EdgeArgument {
                            source: block_id,
                            normal,
                        },
                        definitions,
                        dominators,
                    );
                }
            }
            for (index, operation) in block.operations.iter().enumerate() {
                for value in operation.uses() {
                    self.verify_use(
                        value,
                        UseSite::Operation {
                            block: block_id,
                            index,
                        },
                        definitions,
                        dominators,
                    );
                }
            }
            for value in block.terminator.local_uses() {
                self.verify_use(
                    value,
                    UseSite::Terminator { block: block_id },
                    definitions,
                    dominators,
                );
            }
        }
    }

    /// Panics unless `definition` dominates `usage`, with the extra rule that a
    /// fallible `Try` result is visible only once its normal arm has been taken.
    fn verify_use(
        &self,
        value: ValueId,
        usage: UseSite,
        definitions: &HashMap<ValueId, ValueDefinition>,
        dominators: &HashMap<BlockId, HashSet<BlockId>>,
    ) {
        let Some(&definition) = definitions.get(&value) else {
            panic!("{value} is used at {usage:?} but is not defined");
        };
        let use_block = usage.block();
        let dominates = match definition {
            ValueDefinition::This | ValueDefinition::Parameter(_) => true,
            ValueDefinition::CaughtException(block) => dominators[&use_block].contains(&block),
            ValueDefinition::Instruction(InstructionLocation::Operation {
                block: definition_block,
                index,
            }) if definition_block == use_block => match usage {
                UseSite::Operation {
                    index: use_index, ..
                } => index < use_index,
                UseSite::Terminator { .. } | UseSite::EdgeArgument { .. } => true,
            },
            ValueDefinition::Instruction(InstructionLocation::Terminator {
                block: definition_block,
            }) if definition_block == use_block => matches!(usage, UseSite::EdgeArgument { .. }),
            ValueDefinition::Instruction(location) => {
                dominators[&use_block].contains(&location_block(location))
            }
        };
        assert!(
            dominates,
            "definition of {value} at {definition:?} does not dominate use at {usage:?}"
        );

        let ValueDefinition::Instruction(InstructionLocation::Terminator {
            block: definition_block,
        }) = definition
        else {
            return;
        };
        if let UseSite::EdgeArgument { source, normal } = usage
            && source == definition_block
        {
            assert!(
                normal,
                "fallible result {value} is used as an argument on a non-normal edge"
            );
            return;
        }
        assert!(
            !self
                .reachable_blocks(&HashSet::from([definition_block]))
                .contains(&use_block),
            "fallible result {value} is visible at {usage:?} without taking its normal edge"
        );
    }
}
