use super::*;
use crate::ir::{
    BasicBlock, EdgeId, InstructionId, Successor, Terminator, TerminatorKind,
    control_flow::path_condition::BooleanVariable, expression::Condition,
};

fn block(id: u32, successors: Vec<Successor>) -> BasicBlock {
    BasicBlock::new(
        BlockId::new(id),
        vec![],
        vec![],
        Terminator::new(
            InstructionId::new(id),
            if successors.len() == 2 {
                TerminatorKind::Branch
            } else if successors.is_empty() {
                TerminatorKind::Return(None)
            } else {
                TerminatorKind::Goto
            },
            successors,
        ),
    )
}

#[test]
fn path_conditions_prune_contradictory_arms_at_block_locations() {
    let condition = Condition::IsZero(crate::ir::ValueId::new(0));
    let positive: BooleanVariable<Predicate> = condition.into();
    let negative = !positive.clone();
    let blocks = vec![
        block(
            0,
            vec![Successor::new(
                EdgeId::new(0),
                BlockId::new(1),
                ControlTransfer::Conditional(BranchGuard::of(positive.clone())),
            )],
        ),
        block(
            1,
            vec![
                Successor::new(
                    EdgeId::new(1),
                    BlockId::new(2),
                    ControlTransfer::Conditional(BranchGuard::of(negative)),
                ),
                Successor::new(
                    EdgeId::new(2),
                    BlockId::new(3),
                    ControlTransfer::Unconditional,
                ),
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
                Successor::new(
                    EdgeId::new(0),
                    BlockId::new(1),
                    ControlTransfer::Conditional(BranchGuard::of(positive)),
                ),
                Successor::new(
                    EdgeId::new(1),
                    BlockId::new(5),
                    ControlTransfer::Conditional(BranchGuard::of(negative)),
                ),
            ],
        ),
        block(
            1,
            vec![
                Successor::new(EdgeId::new(2), BlockId::new(2), ControlTransfer::Normal),
                Successor::new(
                    EdgeId::new(3),
                    BlockId::new(3),
                    ControlTransfer::Exception(Some("java/lang/RuntimeException".parse().unwrap())),
                ),
                Successor::new(EdgeId::new(4), BlockId::new(4), ControlTransfer::Unwind),
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
