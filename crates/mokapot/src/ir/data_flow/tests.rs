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
