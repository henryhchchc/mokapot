use super::*;
use crate::{
    ir::{
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, PathValue},
        },
        expression::Predicate,
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
    let method = method(body, "(I)V", vec![]);
    let ir = build(&method).unwrap();
    let switch = &ir.block(ir.entry_block()).unwrap().terminator;

    assert!(matches!(switch, Terminator::Switch { .. }));
    assert_eq!(switch.successors().count(), 3);
    let check = |it| it == switch.successors().next().unwrap().block_target();
    assert!(switch.successors().map(Successor::block_target).all(check));
    assert_eq!(ir.outgoing_edges(ir.entry_block()).count(), 3);
}

#[test]
fn branch_preserves_taken_then_fallthrough_guards() {
    let body = [
        (0, Instruction::ILoad0),
        (1, Instruction::IfEq(5.into())),
        (2, Instruction::Return),
        (5, Instruction::Return),
    ];
    let method = method(body, "(I)V", vec![]);
    let ir = build(&method).unwrap();
    let branch = &ir.block(ir.entry_block()).unwrap().terminator;

    assert!(matches!(branch, Terminator::Branch { .. }));
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
    let method = method(body, "(II)V", vec![]);
    let ir = build(&method).unwrap();
    let branch = &ir.block(ir.entry_block()).unwrap().terminator;
    let parameters = ir.parameter_values();

    let (lhs, rhs) = (
        PathValue::Variable(parameters[0]),
        PathValue::Variable(parameters[1]),
    );
    let predicate = BooleanVariable::Positive(Predicate::LessThan(lhs, rhs));
    let expected = ControlTransfer::Conditional(BranchGuard::of(predicate));
    assert_eq!(
        branch.successors().next().unwrap().transfer(),
        Some(&expected)
    );
}

#[test]
fn tableswitch_preserves_ordered_parallel_arms_and_case_guards() {
    let jump_targets = vec![10.into(), 10.into()];
    let instruction = Instruction::TableSwitch {
        range: 3..=4,
        jump_targets,
        default: 10.into(),
    };
    let body = [
        (0, Instruction::ILoad0),
        (1, instruction),
        (10, Instruction::Return),
    ];
    let method = method(body, "(I)V", vec![]);
    let ir = build(&method).unwrap();
    let switch = &ir.block(ir.entry_block()).unwrap().terminator;
    let match_value = PathValue::Variable(ir.parameter_values()[0]);
    let case_guard = |it| {
        let case = PathValue::Constant(ConstantValue::Integer(it));
        let predicate = BooleanVariable::Positive(Predicate::Equal(match_value.clone(), case));
        ControlTransfer::Conditional(BranchGuard::of(predicate))
    };

    assert!(matches!(switch, Terminator::Switch { .. }));
    let arms = switch.successors().collect::<Vec<_>>();
    assert_eq!(arms.len(), 3);
    let check = |it| it == arms[0].block_target();
    assert!(arms.iter().map(|it| it.block_target()).all(check));
    assert_eq!(arms[0].transfer(), Some(&case_guard(3)));
    assert_eq!(arms[1].transfer(), Some(&case_guard(4)));
    let default = arms[2].transfer().unwrap();
    assert!(matches!(default, ControlTransfer::Conditional(guard) if guard.predicate_count() == 2));
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
    let method = method(body, "(I)V", vec![]);
    let ir = build(&method).unwrap();
    let terminator = &ir.block(ir.entry_block()).unwrap().terminator;

    let successors = terminator.successors().collect::<Vec<_>>();
    assert_eq!(successors.len(), 1);
    let default_target =
        ir.source_map()
            .instructions_at(10.into())
            .find_map(|it| match it {
                InstructionLocation::Terminator { block } => Some(block),
                InstructionLocation::BlockParameter { .. }
                | InstructionLocation::Operation { .. } => None,
            });
    assert_eq!(successors[0].block_target(), default_target);
    assert!(!terminator.uses().contains(&ir.parameter_values()[0]));
    assert_eq!(entry_origin(&ir), Some(1.into()));
}

#[test]
fn fallible_exit_keeps_normal_then_ordered_handler_arms() {
    let table = vec![
        handler(
            1.into()..2.into(),
            10.into(),
            Some("java/lang/RuntimeException".parse().unwrap()),
        ),
        handler(
            1.into()..2.into(),
            20.into(),
            Some("java/lang/Throwable".parse().unwrap()),
        ),
    ];
    let str_type = "java/lang/String".parse().unwrap();
    let body = [
        (0, Instruction::AConstNull),
        (1, Instruction::CheckCast(str_type)),
        (2, Instruction::Pop),
        (3, Instruction::Return),
        (10, Instruction::AStore0),
        (11, Instruction::Return),
        (20, Instruction::AStore0),
        (21, Instruction::Return),
    ];
    let method = method(body, "()V", table);
    let ir = build(&method).unwrap();
    let fallible = &ir.block(ir.entry_block()).unwrap().terminator;

    assert!(matches!(fallible, Terminator::Try { .. }));
    assert_eq!(entry_origin(&ir), Some(1.into()));
    assert!(matches!(
        fallible.successors().next().unwrap().transfer(),
        Some(ControlTransfer::Unconditional)
    ));
    let handler_types = fallible
        .successors()
        .skip(1)
        .map(|it| match it.transfer() {
            Some(ControlTransfer::Exception(Some(caught))) => caught.0.as_ref(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handler_types,
        ["java/lang/RuntimeException", "java/lang/Throwable"]
    );
}

#[test]
fn throw_is_a_source_backed_terminator() {
    let body = [(0, Instruction::ALoad0), (1, Instruction::AThrow)];
    let method = method(body, "(Ljava/lang/Throwable;)V", vec![]);
    let ir = build(&method).unwrap();

    assert!(matches!(
        ir.block(ir.entry_block()).unwrap().terminator,
        Terminator::Throw { .. }
    ));
    assert_eq!(entry_origin(&ir), Some(1.into()));
}
