use super::*;
use crate::ir::{
    BasicBlock, BlockKind, EdgeId, Successor, SuccessorTarget, Terminator,
    control_flow::path_condition::BooleanVariable, expression::Predicate,
};

fn code_bb(id: u32, terminator: Terminator) -> (BlockId, BasicBlock) {
    let bb = BasicBlock {
        kind: BlockKind::Code,
        parameters: vec![],
        operations: vec![],
        terminator,
    };
    (BlockId::new(id), bb)
}

#[test]
fn path_conditions_prune_contradictory_arms_at_block_locations() {
    let condition = Predicate::IsZero(crate::ir::ValueId::new(0).into());
    let positive: BooleanVariable<Predicate> = condition.into();
    let negative = !positive.clone();

    let goto = Terminator::Goto {
        target: Successor {
            id: EdgeId::new(0),
            target: SuccessorTarget::Block(BlockId::new(1)),
            arguments: vec![],
            transfer: ControlTransfer::Conditional(BranchGuard::of(positive.clone())),
        },
    };
    let branch = Terminator::Branch {
        taken: Successor {
            id: EdgeId::new(1),
            target: SuccessorTarget::Block(BlockId::new(2)),
            arguments: vec![],
            transfer: ControlTransfer::Conditional(BranchGuard::of(negative)),
        },
        otherwise: Successor {
            id: EdgeId::new(2),
            target: SuccessorTarget::Block(BlockId::new(3)),
            arguments: vec![],
            transfer: ControlTransfer::Unconditional,
        },
    };
    let exit = Terminator::Return { value: None };

    let blocks = BTreeMap::from([
        code_bb(0, goto),
        code_bb(1, branch),
        code_bb(2, exit.clone()),
        code_bb(3, exit),
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

    let branch = Terminator::Branch {
        taken: Successor {
            id: EdgeId::new(0),
            target: SuccessorTarget::Block(BlockId::new(1)),
            arguments: vec![],
            transfer: ControlTransfer::Conditional(BranchGuard::of(positive)),
        },
        otherwise: Successor {
            id: EdgeId::new(1),
            target: SuccessorTarget::Block(BlockId::new(5)),
            arguments: vec![],
            transfer: ControlTransfer::Conditional(BranchGuard::of(negative)),
        },
    };
    let exception_type = "java/lang/RuntimeException".parse().unwrap();
    let fallible = Terminator::Try {
        operation: crate::ir::Operation::Effect {
            expr: crate::ir::expression::Expression::Const(crate::jvm::ConstantValue::Null),
        },
        normal: Successor {
            id: EdgeId::new(2),
            target: SuccessorTarget::Block(BlockId::new(2)),
            arguments: vec![],
            transfer: ControlTransfer::Unconditional,
        },
        exceptional: vec![
            Successor {
                id: EdgeId::new(3),
                target: SuccessorTarget::Block(BlockId::new(3)),
                arguments: vec![],
                transfer: ControlTransfer::Exception(Some(exception_type)),
            },
            Successor {
                id: EdgeId::new(4),
                target: SuccessorTarget::Unwind,
                arguments: vec![],
                transfer: ControlTransfer::Unwind,
            },
        ],
    };
    let exit = Terminator::Return { value: None };

    let blocks = BTreeMap::from([
        code_bb(0, branch),
        code_bb(1, fallible),
        code_bb(2, exit.clone()),
        code_bb(3, exit.clone()),
        code_bb(5, exit),
    ]);
    let conditions = ControlFlowGraph::new(&blocks, BlockId::new(0)).path_conditions();

    assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(2)]);
    assert_eq!(conditions[&BlockId::new(1)], conditions[&BlockId::new(3)]);
    assert_ne!(conditions[&BlockId::new(1)], conditions[&BlockId::new(5)]);
}
