//! Data flow analysis.

use std::collections::{BTreeSet, HashMap};

use super::{BlockId, InstructionId, MokaIRMethod, ValueDefinition, ValueId};

/// A location at which an SSA value is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UseSite {
    /// An ordinary instruction, terminator, or successor guard.
    Instruction(InstructionId),
    /// An input selected on entry from one predecessor block.
    PhiInput {
        /// The phi instruction consuming the value.
        phi: InstructionId,
        /// The predecessor selecting this input.
        predecessor: BlockId,
    },
}

impl UseSite {
    /// Returns the identified IR node containing this use.
    #[must_use]
    pub const fn instruction(self) -> InstructionId {
        match self {
            Self::Instruction(instruction)
            | Self::PhiInput {
                phi: instruction, ..
            } => instruction,
        }
    }
}

/// A method-local index of SSA definitions and uses.
#[derive(Debug)]
pub struct DefUseChain {
    defs: HashMap<ValueId, ValueDefinition>,
    uses: HashMap<ValueId, BTreeSet<UseSite>>,
}

impl DefUseChain {
    /// Creates a new def-use index from a method.
    #[must_use]
    pub fn new(method: &MokaIRMethod) -> Self {
        let defs = method.value_definitions().collect::<HashMap<_, _>>();
        let mut uses: HashMap<ValueId, BTreeSet<UseSite>> = HashMap::new();

        for block in method.blocks() {
            for phi in block.phis() {
                for input in phi.inputs() {
                    uses.entry(input.value())
                        .or_default()
                        .insert(UseSite::PhiInput {
                            phi: phi.id(),
                            predecessor: input.predecessor(),
                        });
                }
            }
            for instruction in block.instructions() {
                for value in instruction.uses() {
                    uses.entry(value)
                        .or_default()
                        .insert(UseSite::Instruction(instruction.id()));
                }
            }
            for value in block.terminator().uses() {
                uses.entry(value)
                    .or_default()
                    .insert(UseSite::Instruction(block.terminator().id()));
            }
        }

        Self { defs, uses }
    }

    /// Returns the definition of a value.
    #[must_use]
    pub fn defined_at(&self, value: ValueId) -> Option<ValueDefinition> {
        self.defs.get(&value).copied()
    }

    /// Returns the locations where a value is used.
    #[must_use]
    pub fn used_at(&self, value: ValueId) -> BTreeSet<UseSite> {
        self.uses.get(&value).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        ir::{
            BasicBlock, BlockId, EdgeId, InstructionId, Phi, PhiInput, SourceMap, Successor,
            Terminator, TerminatorKind,
            control_flow::{
                ControlTransfer,
                path_condition::{BooleanVariable, BranchGuard},
            },
            expression::{Condition, Predicate},
        },
        jvm::method,
    };

    const THIS: ValueId = ValueId::new(0);
    const PARAMETER: ValueId = ValueId::new(1);
    const CAUGHT: ValueId = ValueId::new(2);
    const MERGED: ValueId = ValueId::new(3);
    const LOOP_CARRIED: ValueId = ValueId::new(4);
    const ENTRY: BlockId = BlockId::new(0);
    const ALTERNATE: BlockId = BlockId::new(1);
    const MERGE: BlockId = BlockId::new(2);
    const HANDLER: BlockId = BlockId::new(3);
    const BRANCH: InstructionId = InstructionId::new(0);
    const PHI: InstructionId = InstructionId::new(2);
    const LOOP_PHI: InstructionId = InstructionId::new(3);
    const MERGE_TERMINATOR: InstructionId = InstructionId::new(4);
    const RETURN: InstructionId = InstructionId::new(5);

    fn method_with_phi_uses() -> MokaIRMethod {
        let condition: BooleanVariable<Predicate> = Condition::IsZero(PARAMETER).into();
        let phi = Phi::new(
            PHI,
            MERGED,
            vec![
                PhiInput::new(ENTRY, THIS),
                PhiInput::new(ALTERNATE, THIS),
                PhiInput::new(MERGE, THIS),
            ],
        );
        let loop_phi = Phi::new(
            LOOP_PHI,
            LOOP_CARRIED,
            vec![
                PhiInput::new(ENTRY, THIS),
                PhiInput::new(ALTERNATE, THIS),
                PhiInput::new(MERGE, LOOP_CARRIED),
            ],
        );
        let entry = BasicBlock::new(
            ENTRY,
            vec![],
            vec![],
            Terminator::new(
                BRANCH,
                TerminatorKind::Branch,
                vec![
                    Successor::new(
                        EdgeId::new(0),
                        MERGE,
                        ControlTransfer::Conditional(BranchGuard::of(condition.clone())),
                    ),
                    Successor::new(
                        EdgeId::new(1),
                        ALTERNATE,
                        ControlTransfer::Conditional(BranchGuard::of(!condition)),
                    ),
                ],
            ),
        );
        let alternate = BasicBlock::new(
            ALTERNATE,
            vec![],
            vec![],
            Terminator::new(
                InstructionId::new(1),
                TerminatorKind::Goto,
                vec![Successor::new(
                    EdgeId::new(2),
                    MERGE,
                    ControlTransfer::Unconditional,
                )],
            ),
        );
        let merge = BasicBlock::new(
            MERGE,
            vec![phi, loop_phi],
            vec![],
            Terminator::new(MERGE_TERMINATOR, TerminatorKind::Branch, {
                let condition: BooleanVariable<Predicate> = Condition::IsZero(MERGED).into();
                vec![
                    Successor::new(
                        EdgeId::new(3),
                        HANDLER,
                        ControlTransfer::Conditional(BranchGuard::of(condition.clone())),
                    ),
                    Successor::new(
                        EdgeId::new(4),
                        MERGE,
                        ControlTransfer::Conditional(BranchGuard::of(!condition)),
                    ),
                ]
            }),
        );
        let handler = BasicBlock::new(
            HANDLER,
            vec![],
            vec![],
            Terminator::new(RETURN, TerminatorKind::Return(Some(PARAMETER)), vec![]),
        );
        MokaIRMethod::new(
            method::AccessFlags::empty(),
            "test".to_owned(),
            "(I)I".parse().unwrap(),
            "org/mokapot/Test".parse().unwrap(),
            ENTRY,
            vec![entry, alternate, merge, handler],
            SourceMap::default(),
            Some(THIS),
            vec![PARAMETER],
            BTreeMap::from([(HANDLER, CAUGHT)]),
            vec![
                ValueDefinition::This,
                ValueDefinition::Parameter(0),
                ValueDefinition::CaughtException(HANDLER),
                ValueDefinition::Instruction(PHI),
                ValueDefinition::Instruction(LOOP_PHI),
            ],
        )
    }

    #[test]
    fn records_entry_phi_and_terminator_data_flow() {
        let method = method_with_phi_uses();

        let chain = DefUseChain::new(&method);

        assert_eq!(chain.defined_at(THIS), Some(ValueDefinition::This));
        assert_eq!(
            chain.defined_at(PARAMETER),
            Some(ValueDefinition::Parameter(0))
        );
        assert_eq!(
            chain.defined_at(CAUGHT),
            Some(ValueDefinition::CaughtException(HANDLER))
        );
        assert_eq!(
            chain.defined_at(MERGED),
            Some(ValueDefinition::Instruction(PHI))
        );
        assert_eq!(
            chain.defined_at(LOOP_CARRIED),
            Some(ValueDefinition::Instruction(LOOP_PHI))
        );
        assert_eq!(
            chain.used_at(THIS),
            BTreeSet::from([
                UseSite::PhiInput {
                    phi: PHI,
                    predecessor: ENTRY,
                },
                UseSite::PhiInput {
                    phi: PHI,
                    predecessor: ALTERNATE,
                },
                UseSite::PhiInput {
                    phi: PHI,
                    predecessor: MERGE,
                },
                UseSite::PhiInput {
                    phi: LOOP_PHI,
                    predecessor: ENTRY,
                },
                UseSite::PhiInput {
                    phi: LOOP_PHI,
                    predecessor: ALTERNATE,
                },
            ])
        );
        assert_eq!(
            chain.used_at(PARAMETER),
            BTreeSet::from([UseSite::Instruction(BRANCH), UseSite::Instruction(RETURN),])
        );
        assert_eq!(
            UseSite::PhiInput {
                phi: PHI,
                predecessor: ENTRY,
            }
            .instruction(),
            PHI
        );
        assert_eq!(
            chain.used_at(MERGED),
            BTreeSet::from([UseSite::Instruction(MERGE_TERMINATOR)])
        );
        assert_eq!(
            chain.used_at(LOOP_CARRIED),
            BTreeSet::from([UseSite::PhiInput {
                phi: LOOP_PHI,
                predecessor: MERGE,
            }])
        );
        assert!(chain.used_at(CAUGHT).is_empty());
    }
}
