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

fn merge_phi(id: InstructionId, value: ValueId, merge_value: ValueId) -> Phi {
    Phi {
        id,
        value,
        inputs: vec![
            PhiInput {
                predecessor: ENTRY,
                value: THIS,
            },
            PhiInput {
                predecessor: ALTERNATE,
                value: THIS,
            },
            PhiInput {
                predecessor: MERGE,
                value: merge_value,
            },
        ],
    }
}

fn method_with_phi_uses() -> MokaIRMethod {
    let condition: BooleanVariable<Predicate> = Condition::IsZero(PARAMETER).into();
    let phi = merge_phi(PHI, MERGED, THIS);
    let loop_phi = merge_phi(LOOP_PHI, LOOP_CARRIED, LOOP_CARRIED);
    let entry = BasicBlock {
        id: ENTRY,
        phis: vec![],
        operations: vec![],
        terminator: Terminator {
            id: BRANCH,
            kind: TerminatorKind::Branch,
            successors: vec![
                Successor {
                    id: EdgeId::new(0),
                    target: MERGE,
                    transfer: ControlTransfer::Conditional(BranchGuard::of(condition.clone())),
                },
                Successor {
                    id: EdgeId::new(1),
                    target: ALTERNATE,
                    transfer: ControlTransfer::Conditional(BranchGuard::of(!condition)),
                },
            ],
        },
    };
    let alternate = BasicBlock {
        id: ALTERNATE,
        phis: vec![],
        operations: vec![],
        terminator: Terminator {
            id: InstructionId::new(1),
            kind: TerminatorKind::Goto,
            successors: vec![Successor {
                id: EdgeId::new(2),
                target: MERGE,
                transfer: ControlTransfer::Unconditional,
            }],
        },
    };
    let merge = BasicBlock {
        id: MERGE,
        phis: vec![phi, loop_phi],
        operations: vec![],
        terminator: Terminator {
            id: MERGE_TERMINATOR,
            kind: TerminatorKind::Branch,
            successors: {
                let condition: BooleanVariable<Predicate> = Condition::IsZero(MERGED).into();
                vec![
                    Successor {
                        id: EdgeId::new(3),
                        target: HANDLER,
                        transfer: ControlTransfer::Conditional(BranchGuard::of(condition.clone())),
                    },
                    Successor {
                        id: EdgeId::new(4),
                        target: MERGE,
                        transfer: ControlTransfer::Conditional(BranchGuard::of(!condition)),
                    },
                ]
            },
        },
    };
    let handler = BasicBlock {
        id: HANDLER,
        phis: vec![],
        operations: vec![],
        terminator: Terminator {
            id: RETURN,
            kind: TerminatorKind::Return(Some(PARAMETER)),
            successors: vec![],
        },
    };
    MokaIRMethod {
        access_flags: method::AccessFlags::empty(),
        name: "test".to_owned(),
        descriptor: "(I)I".parse().unwrap(),
        owner: "org/mokapot/Test".parse().unwrap(),
        entry_block: ENTRY,
        blocks: vec![entry, alternate, merge, handler],
        source_map: SourceMap::default(),
        this_value: Some(THIS),
        parameter_values: vec![PARAMETER],
        caught_exceptions: BTreeMap::from([(HANDLER, CAUGHT)]),
        value_definitions: vec![
            ValueDefinition::This,
            ValueDefinition::Parameter(0),
            ValueDefinition::CaughtException(HANDLER),
            ValueDefinition::Instruction(PHI),
            ValueDefinition::Instruction(LOOP_PHI),
        ],
    }
}

#[test]
fn records_entry_phi_and_terminator_data_flow() {
    let method = method_with_phi_uses();

    let chain = DefUseChain::new(&method);

    assert_eq!(chain.definition_of(THIS), Some(ValueDefinition::This));
    assert_eq!(
        chain.definition_of(PARAMETER),
        Some(ValueDefinition::Parameter(0))
    );
    assert_eq!(
        chain.definition_of(CAUGHT),
        Some(ValueDefinition::CaughtException(HANDLER))
    );
    assert_eq!(
        chain.definition_of(MERGED),
        Some(ValueDefinition::Instruction(PHI))
    );
    assert_eq!(
        chain.definition_of(LOOP_CARRIED),
        Some(ValueDefinition::Instruction(LOOP_PHI))
    );
    assert_eq!(
        chain.uses_of(THIS).collect::<BTreeSet<_>>(),
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
        chain.uses_of(PARAMETER).collect::<BTreeSet<_>>(),
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
        chain.uses_of(MERGED).collect::<BTreeSet<_>>(),
        BTreeSet::from([UseSite::Instruction(MERGE_TERMINATOR)])
    );
    assert_eq!(
        chain.uses_of(LOOP_CARRIED).collect::<BTreeSet<_>>(),
        BTreeSet::from([UseSite::PhiInput {
            phi: LOOP_PHI,
            predecessor: MERGE,
        }])
    );
    assert_eq!(chain.uses_of(CAUGHT).count(), 0);
}
