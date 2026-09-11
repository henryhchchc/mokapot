use super::*;
use crate::ir::{
    BasicBlock, EdgeId, InstructionId, Successor, Terminator, TerminatorKind,
    control_flow::path_condition::BooleanVariable, expression::Condition,
};

fn block(id: u32, successors: Vec<Successor>) -> BasicBlock {
    BasicBlock {
        id: BlockId::new(id),
        phis: vec![],
        operations: vec![],
        terminator: Terminator {
            id: InstructionId::new(id),
            kind: if successors.len() == 2 {
                TerminatorKind::Branch
            } else if successors.is_empty() {
                TerminatorKind::Return(None)
            } else {
                TerminatorKind::Goto
            },
            successors,
        },
    }
}

#[test]
fn path_conditions_prune_contradictory_arms_at_block_locations() {
    let condition = Condition::IsZero(crate::ir::ValueId::new(0));
    let positive: BooleanVariable<Predicate> = condition.into();
    let negative = !positive.clone();
    let blocks = vec![
        block(
            0,
            vec![Successor {
                id: EdgeId::new(0),
                target: BlockId::new(1),
                transfer: ControlTransfer::Conditional(BranchGuard::of(positive.clone())),
            }],
        ),
        block(
            1,
            vec![
                Successor {
                    id: EdgeId::new(1),
                    target: BlockId::new(2),
                    transfer: ControlTransfer::Conditional(BranchGuard::of(negative)),
                },
                Successor {
                    id: EdgeId::new(2),
                    target: BlockId::new(3),
                    transfer: ControlTransfer::Unconditional,
                },
            ],
        ),
        block(2, vec![]),
        block(3, vec![]),
    ];
    let conditions = ControlFlowGraph::new(&blocks, BlockId::new(0)).path_conditions();

    assert!(conditions.contains_key(&BlockId::new(0)));
    assert!(conditions.contains_key(&BlockId::new(1)));
    assert!(!conditions.contains_key(&BlockId::new(2)));
    assert!(conditions.contains_key(&BlockId::new(3)));
}

#[test]
fn exceptional_outcomes_preserve_the_incoming_path_condition() {
    let condition = Condition::IsZero(crate::ir::ValueId::new(0));
    let positive: BooleanVariable<Predicate> = condition.into();
    let negative = !positive.clone();
    let blocks = vec![
        block(
            0,
            vec![
                Successor {
                    id: EdgeId::new(0),
                    target: BlockId::new(1),
                    transfer: ControlTransfer::Conditional(BranchGuard::of(positive)),
                },
                Successor {
                    id: EdgeId::new(1),
                    target: BlockId::new(5),
                    transfer: ControlTransfer::Conditional(BranchGuard::of(negative)),
                },
            ],
        ),
        block(
            1,
            vec![
                Successor {
                    id: EdgeId::new(2),
                    target: BlockId::new(2),
                    transfer: ControlTransfer::Normal,
                },
                Successor {
                    id: EdgeId::new(3),
                    target: BlockId::new(3),
                    transfer: ControlTransfer::Exception(Some(
                        "java/lang/RuntimeException".parse().unwrap(),
                    )),
                },
                Successor {
                    id: EdgeId::new(4),
                    target: BlockId::new(4),
                    transfer: ControlTransfer::Unwind,
                },
            ],
        ),
        block(2, vec![]),
        block(3, vec![]),
        block(4, vec![]),
        block(5, vec![]),
    ];
    let conditions = ControlFlowGraph::new(&blocks, BlockId::new(0)).path_conditions();

    assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(2)]);
    assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(3)]);
    assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(4)]);
    assert_ne!(conditions[&BlockId::new(1)], conditions[&BlockId::new(5)]);
}
