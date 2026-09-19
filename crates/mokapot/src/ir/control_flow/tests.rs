use super::*;
use crate::ir::{
    BasicBlock, EdgeId, Successor, Terminator, TerminatorKind,
    control_flow::path_condition::BooleanVariable, expression::Predicate,
};

fn block(id: u32, successors: Vec<Successor>) -> (BlockId, BasicBlock) {
    (
        BlockId::new(id),
        BasicBlock {
            caught_exception: None,
            parameters: vec![],
            operations: vec![],
            terminator: Terminator {
                kind: if successors.len() == 2 {
                    TerminatorKind::Branch
                } else if successors.is_empty() {
                    TerminatorKind::Return(None)
                } else {
                    TerminatorKind::Goto
                },
                successors,
            },
        },
    )
}

#[test]
fn path_conditions_prune_contradictory_arms_at_block_locations() {
    let condition = Predicate::IsZero(crate::ir::ValueId::new(0).into());
    let positive: BooleanVariable<Predicate> = condition.into();
    let negative = !positive.clone();
    let blocks = BTreeMap::from([
        block(
            0,
            vec![Successor {
                id: EdgeId::new(0),
                target: BlockId::new(1),
                arguments: vec![],
                transfer: ControlTransfer::Conditional(BranchGuard::of(positive.clone())),
            }],
        ),
        block(
            1,
            vec![
                Successor {
                    id: EdgeId::new(1),
                    target: BlockId::new(2),
                    arguments: vec![],
                    transfer: ControlTransfer::Conditional(BranchGuard::of(negative)),
                },
                Successor {
                    id: EdgeId::new(2),
                    target: BlockId::new(3),
                    arguments: vec![],
                    transfer: ControlTransfer::Unconditional,
                },
            ],
        ),
        block(2, vec![]),
        block(3, vec![]),
    ]);
    let conditions = ControlFlowGraph::new(&blocks, BlockId::new(0)).path_conditions();

    assert!(conditions.contains_key(&BlockId::new(0)));
    assert!(conditions.contains_key(&BlockId::new(1)));
    assert!(!conditions.contains_key(&BlockId::new(2)));
    assert!(conditions.contains_key(&BlockId::new(3)));
}

#[test]
fn exceptional_outcomes_preserve_the_incoming_path_condition() {
    let condition = Predicate::IsZero(crate::ir::ValueId::new(0).into());
    let positive: BooleanVariable<Predicate> = condition.into();
    let negative = !positive.clone();
    let blocks = BTreeMap::from([
        block(
            0,
            vec![
                Successor {
                    id: EdgeId::new(0),
                    target: BlockId::new(1),
                    arguments: vec![],
                    transfer: ControlTransfer::Conditional(BranchGuard::of(positive)),
                },
                Successor {
                    id: EdgeId::new(1),
                    target: BlockId::new(5),
                    arguments: vec![],
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
                    arguments: vec![],
                    transfer: ControlTransfer::Unconditional,
                },
                Successor {
                    id: EdgeId::new(3),
                    target: BlockId::new(3),
                    arguments: vec![],
                    transfer: ControlTransfer::Exception(Some(
                        "java/lang/RuntimeException".parse().unwrap(),
                    )),
                },
                Successor {
                    id: EdgeId::new(4),
                    target: BlockId::new(4),
                    arguments: vec![],
                    transfer: ControlTransfer::Unwind,
                },
            ],
        ),
        block(2, vec![]),
        block(3, vec![]),
        block(4, vec![]),
        block(5, vec![]),
    ]);
    let conditions = ControlFlowGraph::new(&blocks, BlockId::new(0)).path_conditions();

    assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(2)]);
    assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(3)]);
    assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(4)]);
    assert_ne!(conditions[&BlockId::new(1)], conditions[&BlockId::new(5)]);
}
