use super::*;
use crate::jvm::ConstantPool;
use crate::{
    ir::{
        BranchGuard, ControlTransfer,
        expression::{BooleanVariable, PathValue, Predicate},
    },
    jvm::ConstantValue,
};

#[test]
fn branch_preserves_taken_then_fallthrough_guards() {
    let body = [
        (0, InstructionData::ILoad0),
        (1, InstructionData::IfEq(5.into())),
        (2, InstructionData::Return),
        (5, InstructionData::Return),
    ];
    let ir = lift(body, "(I)V", vec![]);
    let branch = &ir.block(ir.entry.block).unwrap().terminator;

    assert_matches!(branch, Terminator::Branch { .. });
    assert_eq!(branch.successors().count(), 2);
    let taken = branch.successors().next().unwrap().transfer().unwrap();
    let otherwise = branch.successors().nth(1).unwrap().transfer().unwrap();
    let is_zero = Predicate::IsZero(PathValue::Variable(ir.parameters[0]));
    let positive = BranchGuard::of(BooleanVariable::Positive(is_zero.clone()));
    let negative = BranchGuard::of(BooleanVariable::Negative(is_zero));
    assert_eq!(taken, &ControlTransfer::Conditional(positive));
    assert_eq!(otherwise, &ControlTransfer::Conditional(negative));
    assert_eq!(entry_origin(&ir), Some(1.into()));
}

#[test]
fn comparison_branch_preserves_operand_order() {
    let body = [
        (0, InstructionData::ILoad0),
        (1, InstructionData::ILoad1),
        (2, InstructionData::IfICmpLt(6.into())),
        (5, InstructionData::Return),
        (6, InstructionData::Return),
    ];
    let ir = lift(body, "(II)V", vec![]);
    let branch = &ir.block(ir.entry.block).unwrap().terminator;

    let (lhs, rhs) = (
        PathValue::Variable(ir.parameters[0]),
        PathValue::Variable(ir.parameters[1]),
    );
    let predicate = BooleanVariable::Positive(Predicate::LessThan(lhs, rhs));
    let expected = ControlTransfer::Conditional(BranchGuard::of(predicate));

    let taken = branch.successors().next().unwrap().transfer();
    assert_eq!(taken, Some(&expected));
}

#[test]
fn switches_preserve_ordered_parallel_arms_and_case_guards() {
    let table = InstructionData::TableSwitch {
        low: 3,
        jump_targets: Box::new([10.into(), 10.into()]),
        default: 10.into(),
    };
    let lookup = InstructionData::LookupSwitch {
        default: 10.into(),
        match_targets: Box::new([(1, 10.into()), (5, 10.into())]),
    };
    for (instruction, cases) in [(table, [3, 4]), (lookup, [1, 5])] {
        let body = [
            (0, InstructionData::ILoad0),
            (1, instruction),
            (10, InstructionData::Return),
        ];
        let ir = lift(body, "(I)V", vec![]);
        let switch = &ir.block(ir.entry.block).unwrap().terminator;
        let match_value = PathValue::Variable(ir.parameters[0]);
        let case_guard = |it| {
            let case = PathValue::Constant(ConstantValue::Integer(it));
            let predicate = BooleanVariable::Positive(Predicate::Equal(match_value.clone(), case));
            ControlTransfer::Conditional(BranchGuard::of(predicate))
        };

        assert_matches!(switch, Terminator::Switch { .. });
        let arms = switch.successors().collect::<Vec<_>>();
        assert_eq!(arms.len(), 3);
        let target = arms[0].block_target().unwrap();
        assert!(arms.iter().all(|arm| arm.block_target() == Some(target)));
        for (arm, case) in arms.iter().zip(cases) {
            assert_eq!(arm.transfer(), Some(&case_guard(case)));
        }
        let default = arms[2].transfer().unwrap();
        assert_matches!(default, ControlTransfer::Conditional(guard) if guard.predicate_count() == 2);
        assert_eq!(entry_origin(&ir), Some(1.into()));
    }
}

#[test]
fn empty_switch_transfers_only_to_the_default_without_using_match_value() {
    let match_targets = Box::new([]);
    let instruction = InstructionData::LookupSwitch {
        default: 10.into(),
        match_targets,
    };
    let body = [
        (0, InstructionData::ILoad0),
        (1, instruction),
        (10, InstructionData::Return),
    ];
    let ir = lift(body, "(I)V", vec![]);
    let terminator = &ir.block(ir.entry.block).unwrap().terminator;

    let successors = terminator.successors().collect::<Vec<_>>();
    assert_eq!(successors.len(), 1);
    let mut locations = ir.source_map.instructions_at(10.into());
    let default_target = locations.find_map(|it| match it {
        InstructionLocation::Terminator { block } => Some(block),
        _ => None,
    });
    assert_eq!(successors[0].block_target(), default_target);
    assert_eq!(
        successors[0].transfer(),
        Some(&ControlTransfer::Unconditional)
    );
    assert_eq!(successors[0].arguments(), []);
    assert_eq!(entry_origin(&ir), Some(1.into()));
}

#[test]
fn fallible_exit_keeps_normal_then_ordered_handler_arms() {
    let runtime = cls_name("java/lang/RuntimeException");
    let throwable = cls_name("java/lang/Throwable");
    let table = vec![
        handler(1.into()..2.into(), 10.into(), Some(runtime)),
        handler(1.into()..2.into(), 20.into(), Some(throwable)),
    ];
    let mut pool = ConstantPool::new();
    let string_type = pool.insert_reference_type(ref_t("java/lang/String"));
    let body = [
        (0, InstructionData::AConstNull),
        (1, InstructionData::CheckCast(string_type)),
        (2, InstructionData::Pop),
        (3, InstructionData::Return),
        (10, InstructionData::AStore0),
        (11, InstructionData::Return),
        (20, InstructionData::AStore0),
        (21, InstructionData::Return),
    ];
    let ir = lift_with_pool(pool, body, "()V", table);
    let fallible = &ir.block(ir.entry.block).unwrap().terminator;

    assert_matches!(fallible, Terminator::Try { .. });
    assert_eq!(entry_origin(&ir), Some(1.into()));
    let first_transfer = fallible.successors().next().unwrap().transfer();
    assert_matches!(first_transfer, Some(ControlTransfer::Unconditional));
    let handler_types = fallible
        .successors()
        .skip(1)
        .map(|it| match it.transfer() {
            Some(ControlTransfer::Exception(Some(caught))) => caught.as_java_str(),
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    let expected = ["java/lang/RuntimeException", "java/lang/Throwable"];
    assert_eq!(handler_types, expected);
}
