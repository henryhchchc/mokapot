use super::*;
use crate::ir::{
    BasicBlock, BlockKind, NumericalId, Successor, Terminator,
    control_flow::path_condition::BooleanVariable, expression::Predicate,
};
use std::collections::HashMap;

fn code_bb(id: u32, terminator: Terminator) -> (BlockId, BasicBlock) {
    let bb = BasicBlock {
        kind: BlockKind::Code,
        parameters: vec![],
        operations: vec![],
        terminator,
    };
    (BlockId::from_raw(id), bb)
}

#[test]
fn path_conditions_prune_contradictory_arms_at_block_locations() {
    let condition = Predicate::IsZero(crate::ir::ValueId::from_raw(0).into());
    let positive: BooleanVariable<Predicate> = condition.into();
    let negative = !positive.clone();

    let goto = Terminator::Goto {
        target: Successor::Block {
            target: BlockId::from_raw(1),
            arguments: vec![],
            transfer: ControlTransfer::Conditional(BranchGuard::of(positive.clone())),
        },
    };
    let branch = Terminator::Branch {
        taken: Successor::Block {
            target: BlockId::from_raw(2),
            arguments: vec![],
            transfer: ControlTransfer::Conditional(BranchGuard::of(negative)),
        },
        otherwise: Successor::Block {
            target: BlockId::from_raw(3),
            arguments: vec![],
            transfer: ControlTransfer::Unconditional,
        },
    };
    let exit = Terminator::Return { value: None };

    let blocks = HashMap::from([
        code_bb(0, goto),
        code_bb(1, branch),
        code_bb(2, exit.clone()),
        code_bb(3, exit),
    ]);
    let conditions = path_condition::analyze(
        &blocks,
        BlockId::from_raw(0),
        path_condition::SolvingBudget::default(),
    );

    assert!(conditions.contains_key(&BlockId::from_raw(0)));
    assert!(conditions.contains_key(&BlockId::from_raw(1)));
    assert!(!conditions.contains_key(&BlockId::from_raw(2)));
    assert!(conditions.contains_key(&BlockId::from_raw(3)));
}

#[test]
fn exceptional_outcomes_preserve_the_incoming_path_condition() {
    let condition = Predicate::IsZero(crate::ir::ValueId::from_raw(0).into());
    let positive: BooleanVariable<Predicate> = condition.into();
    let negative = !positive.clone();

    let branch = Terminator::Branch {
        taken: Successor::Block {
            target: BlockId::from_raw(1),
            arguments: vec![],
            transfer: ControlTransfer::Conditional(BranchGuard::of(positive)),
        },
        otherwise: Successor::Block {
            target: BlockId::from_raw(5),
            arguments: vec![],
            transfer: ControlTransfer::Conditional(BranchGuard::of(negative)),
        },
    };
    let exception_type = "java/lang/RuntimeException".parse().unwrap();
    let fallible = Terminator::Try {
        operation: crate::ir::Operation::Effect {
            expr: crate::ir::expression::Expression::Const(crate::jvm::ConstantValue::Null),
        },
        normal: Successor::Block {
            target: BlockId::from_raw(2),
            arguments: vec![],
            transfer: ControlTransfer::Unconditional,
        },
        exceptional: vec![
            Successor::Block {
                target: BlockId::from_raw(3),
                arguments: vec![],
                transfer: ControlTransfer::Exception(Some(exception_type)),
            },
            Successor::Unwind,
        ],
    };
    let exit = Terminator::Return { value: None };

    let blocks = HashMap::from([
        code_bb(0, branch),
        code_bb(1, fallible),
        code_bb(2, exit.clone()),
        code_bb(3, exit.clone()),
        code_bb(5, exit),
    ]);
    let conditions = path_condition::analyze(
        &blocks,
        BlockId::from_raw(0),
        path_condition::SolvingBudget::default(),
    );

    assert_eq!(
        conditions[&BlockId::from_raw(1)],
        conditions[&BlockId::from_raw(2)]
    );
    assert_eq!(
        conditions[&BlockId::from_raw(1)],
        conditions[&BlockId::from_raw(3)]
    );
    assert_ne!(
        conditions[&BlockId::from_raw(1)],
        conditions[&BlockId::from_raw(5)]
    );
}
