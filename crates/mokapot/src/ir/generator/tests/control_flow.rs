use std::collections::BTreeMap;

use super::*;
use crate::{
    ir::{
        BranchGuard, ControlTransfer,
        expression::{BooleanVariable, PathValue, Predicate},
    },
    jvm::ConstantValue,
};

#[test]
fn switch_retains_parallel_successor_arms() {
    let match_targets = BTreeMap::from([(1, 10.into()), (2, 10.into())]);
    let instruction = Instruction::LookupSwitch {
        default: 10.into(),
        match_targets,
    };
    let body = [
        (0, Instruction::ILoad0),
        (1, instruction),
        (10, Instruction::Return),
    ];
    let ir = lift(body, "(I)V", vec![]);
    let switch = &ir.block(ir.entry_block()).unwrap().terminator;

    assert_matches!(switch, Terminator::Switch { .. });
    let targets = switch
        .successors()
        .map(Successor::block_target)
        .collect::<Vec<_>>();
    // Parallel arms are retained: three arms, one target.
    assert_eq!(targets.len(), 3);
    assert!(targets.iter().all(|it| *it == targets[0]));
    assert_eq!(
        switch
            .successors()
            .filter_map(Successor::block_target)
            .count(),
        3
    );
}

#[test]
fn branch_preserves_taken_then_fallthrough_guards() {
    let body = [
        (0, Instruction::ILoad0),
        (1, Instruction::IfEq(5.into())),
        (2, Instruction::Return),
        (5, Instruction::Return),
    ];
    let ir = lift(body, "(I)V", vec![]);
    let branch = &ir.block(ir.entry_block()).unwrap().terminator;

    assert_matches!(branch, Terminator::Branch { .. });
    assert_eq!(branch.successors().count(), 2);
    let taken = branch.successors().next().unwrap().transfer().unwrap();
    let otherwise = branch.successors().nth(1).unwrap().transfer().unwrap();
    let is_zero = Predicate::IsZero(PathValue::Variable(ir.parameter_values()[0]));
    let positive = BranchGuard::of(BooleanVariable::Positive(is_zero.clone()));
    let negative = BranchGuard::of(BooleanVariable::Negative(is_zero));
    assert_eq!(taken, &ControlTransfer::Conditional(positive));
    assert_eq!(otherwise, &ControlTransfer::Conditional(negative));
    assert_eq!(entry_origin(&ir), Some(1.into()));
}

#[test]
fn comparison_branch_preserves_operand_order() {
    let body = [
        (0, Instruction::ILoad0),
        (1, Instruction::ILoad1),
        (2, Instruction::IfICmpLt(6.into())),
        (5, Instruction::Return),
        (6, Instruction::Return),
    ];
    let ir = lift(body, "(II)V", vec![]);
    let branch = &ir.block(ir.entry_block()).unwrap().terminator;
    let parameters = ir.parameter_values();

    let (lhs, rhs) = (
        PathValue::Variable(parameters[0]),
        PathValue::Variable(parameters[1]),
    );
    let predicate = BooleanVariable::Positive(Predicate::LessThan(lhs, rhs));
    let expected = ControlTransfer::Conditional(BranchGuard::of(predicate));

    let taken = branch.successors().next().unwrap().transfer();
    assert_eq!(taken, Some(&expected));
}

#[test]
fn tableswitch_preserves_ordered_parallel_arms_and_case_guards() {
    let jump_targets = vec![10.into(), 10.into()];
    let instruction = Instruction::TableSwitch {
        low: 3,
        jump_targets,
        default: 10.into(),
    };
    let body = [
        (0, Instruction::ILoad0),
        (1, instruction),
        (10, Instruction::Return),
    ];
    let ir = lift(body, "(I)V", vec![]);
    let switch = &ir.block(ir.entry_block()).unwrap().terminator;
    let match_value = PathValue::Variable(ir.parameter_values()[0]);
    let case_guard = |it| {
        let case = PathValue::Constant(ConstantValue::Integer(it));
        let predicate = BooleanVariable::Positive(Predicate::Equal(match_value.clone(), case));
        ControlTransfer::Conditional(BranchGuard::of(predicate))
    };

    assert_matches!(switch, Terminator::Switch { .. });
    let arms = switch.successors().collect::<Vec<_>>();
    assert_eq!(arms.len(), 3);
    let check = |it| it == arms[0].block_target();
    assert!(arms.iter().map(|it| it.block_target()).all(check));
    assert_eq!(arms[0].transfer(), Some(&case_guard(3)));
    assert_eq!(arms[1].transfer(), Some(&case_guard(4)));
    let default = arms[2].transfer().unwrap();
    assert_matches!(default, ControlTransfer::Conditional(guard) if guard.predicate_count() == 2);
    assert_eq!(entry_origin(&ir), Some(1.into()));
}

#[test]
fn empty_switch_transfers_only_to_the_default_without_using_match_value() {
    let match_targets = BTreeMap::new();
    let instruction = Instruction::LookupSwitch {
        default: 10.into(),
        match_targets,
    };
    let body = [
        (0, Instruction::ILoad0),
        (1, instruction),
        (10, Instruction::Return),
    ];
    let ir = lift(body, "(I)V", vec![]);
    let terminator = &ir.block(ir.entry_block()).unwrap().terminator;

    let successors = terminator.successors().collect::<Vec<_>>();
    assert_eq!(successors.len(), 1);
    let mut locations = ir.source_map().instructions_at(10.into());
    let default_target = locations.find_map(|it| match it {
        InstructionLocation::Terminator { block } => Some(block),
        _ => None,
    });
    assert_eq!(successors[0].block_target(), default_target);
    assert!(!terminator.uses().contains(&ir.parameter_values()[0]));
    assert_eq!(entry_origin(&ir), Some(1.into()));
}

#[test]
fn fallible_exit_keeps_normal_then_ordered_handler_arms() {
    let runtime = cls_r("java/lang/RuntimeException");
    let throwable = cls_r("java/lang/Throwable");
    let table = vec![
        handler(1.into()..2.into(), 10.into(), Some(runtime)),
        handler(1.into()..2.into(), 20.into(), Some(throwable)),
    ];
    let body = [
        (0, Instruction::AConstNull),
        (1, Instruction::CheckCast(ref_t("java/lang/String"))),
        (2, Instruction::Pop),
        (3, Instruction::Return),
        (10, Instruction::AStore0),
        (11, Instruction::Return),
        (20, Instruction::AStore0),
        (21, Instruction::Return),
    ];
    let ir = lift(body, "()V", table);
    let fallible = &ir.block(ir.entry_block()).unwrap().terminator;

    assert_matches!(fallible, Terminator::Try { .. });
    assert_eq!(entry_origin(&ir), Some(1.into()));
    let first_transfer = fallible.successors().next().unwrap().transfer();
    assert_matches!(first_transfer, Some(ControlTransfer::Unconditional));
    let handler_types = fallible
        .successors()
        .skip(1)
        .map(|it| match it.transfer() {
            Some(ControlTransfer::Exception(Some(caught))) => caught.0.as_ref(),
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    let expected = ["java/lang/RuntimeException", "java/lang/Throwable"];
    assert_eq!(handler_types, expected);
}

#[test]
fn loop_header_takes_a_block_argument_from_its_back_edge() {
    let body = [
        (0, Instruction::IConst0),
        (1, Instruction::IStore1),
        (2, Instruction::ILoad1),
        (3, Instruction::ILoad0),
        (4, Instruction::IfICmpGe(7.into())),
        (5, Instruction::IInc(1, 1)),
        (6, Instruction::Goto(2.into())),
        (7, Instruction::ILoad1),
        (8, Instruction::IReturn),
    ];
    let ir = lift(body, "(I)I", vec![]);
    let bb = ir.block(ir.entry_block()).unwrap();
    let next_succ = bb.terminator.successors().next().unwrap();
    let header = next_succ.block_target().expect("the entry falls through");

    let header = ir.block(header).unwrap();
    let [param] = header.parameters.as_slice() else {
        panic!("the loop header merges its counter through exactly one block argument");
    };
    assert!(
        header.terminator.uses().contains(&param.value),
        "the merged counter must feed the loop condition"
    );
}

#[test]
fn throw_is_a_source_backed_terminator() {
    let body = [(0, Instruction::ALoad0), (1, Instruction::AThrow)];
    let ir = lift(body, "(Ljava/lang/Throwable;)V", vec![]);
    let terminator = &ir.block(ir.entry_block()).unwrap().terminator;
    assert_matches!(terminator, Terminator::Throw { .. });
    assert_eq!(entry_origin(&ir), Some(1.into()));
}
