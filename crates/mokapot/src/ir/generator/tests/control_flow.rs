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
    let method = method(
        [
            (0, Instruction::ILoad0),
            (
                1,
                Instruction::LookupSwitch {
                    default: 10.into(),
                    match_targets: BTreeMap::from([(1, 10.into()), (2, 10.into())]),
                },
            ),
            (10, Instruction::Return),
        ],
        "(I)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let switch = &ir.block(ir.entry_block()).unwrap().terminator;

    assert!(matches!(switch, Terminator::Switch { .. }));
    assert_eq!(switch.successors().count(), 3);
    assert_eq!(
        switch
            .successors()
            .map(Successor::id)
            .collect::<HashSet<_>>()
            .len(),
        3
    );
    assert!(
        switch
            .successors()
            .map(Successor::block_target)
            .all(|target| target == switch.successors().next().unwrap().block_target())
    );
    assert_eq!(ir.control_flow_graph().edges().count(), 3);
}

#[test]
fn branch_preserves_taken_then_fallthrough_guards() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::IfEq(5.into())),
            (2, Instruction::Return),
            (5, Instruction::Return),
        ],
        "(I)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let branch = &ir.block(ir.entry_block()).unwrap().terminator;
    let match_value = ir.parameter_values()[0];

    assert!(matches!(branch, Terminator::Branch { .. }));
    assert_eq!(branch.successors().count(), 2);
    assert_eq!(
        branch.successors().next().unwrap().transfer(),
        Some(&ControlTransfer::Conditional(BranchGuard::of(
            BooleanVariable::Positive(Predicate::IsZero(PathValue::Variable(match_value)),)
        )))
    );
    assert_eq!(
        branch.successors().nth(1).unwrap().transfer(),
        Some(&ControlTransfer::Conditional(BranchGuard::of(
            BooleanVariable::Negative(Predicate::IsZero(PathValue::Variable(match_value)),)
        )))
    );
    let entry_loc = InstructionLocation::Terminator {
        block: ir.entry_block(),
    };
    assert_eq!(
        ir.source_map().origin_of(entry_loc),
        Some(ProgramCounter::from(1))
    );
}

#[test]
fn comparison_branch_preserves_operand_order() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::ILoad1),
            (2, Instruction::IfICmpLt(6.into())),
            (5, Instruction::Return),
            (6, Instruction::Return),
        ],
        "(II)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let branch = &ir.block(ir.entry_block()).unwrap().terminator;
    let parameters = ir.parameter_values();

    assert_eq!(
        branch.successors().next().unwrap().transfer(),
        Some(&ControlTransfer::Conditional(BranchGuard::of(
            BooleanVariable::Positive(Predicate::LessThan(
                PathValue::Variable(parameters[0]),
                PathValue::Variable(parameters[1]),
            ),)
        )))
    );
}

#[test]
fn tableswitch_preserves_ordered_parallel_arms_and_case_guards() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (
                1,
                Instruction::TableSwitch {
                    range: 3..=4,
                    jump_targets: vec![10.into(), 10.into()],
                    default: 10.into(),
                },
            ),
            (10, Instruction::Return),
        ],
        "(I)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let switch = &ir.block(ir.entry_block()).unwrap().terminator;
    let match_value = ir.parameter_values()[0];
    let case_guard = |case| {
        ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(
            Predicate::Equal(
                PathValue::Variable(match_value),
                PathValue::Constant(ConstantValue::Integer(case)),
            ),
        )))
    };

    assert!(matches!(switch, Terminator::Switch { .. }));
    assert_eq!(switch.successors().count(), 3);
    assert!(
        switch
            .successors()
            .map(Successor::block_target)
            .all(|target| target == switch.successors().next().unwrap().block_target())
    );
    assert_eq!(
        switch.successors().next().unwrap().transfer(),
        Some(&case_guard(3))
    );
    assert_eq!(
        switch.successors().nth(1).unwrap().transfer(),
        Some(&case_guard(4))
    );
    assert!(matches!(
        switch.successors().nth(2).unwrap().transfer(),
        Some(ControlTransfer::Conditional(guard)) if guard.predicate_count() == 2
    ));
    let entry_loc = InstructionLocation::Terminator {
        block: ir.entry_block(),
    };
    assert_eq!(
        ir.source_map().origin_of(entry_loc),
        Some(ProgramCounter::from(1))
    );
}

#[test]
fn empty_switch_transfers_only_to_the_default_without_using_match_value() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (
                1,
                Instruction::LookupSwitch {
                    default: 10.into(),
                    match_targets: BTreeMap::new(),
                },
            ),
            (10, Instruction::Return),
        ],
        "(I)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let terminator = &ir.block(ir.entry_block()).unwrap().terminator;
    let match_value = ir.parameter_values()[0];

    let successors = terminator.successors().collect::<Vec<_>>();
    assert_eq!(successors.len(), 1);
    assert_eq!(
        successors[0].block_target(),
        ir.source_map()
            .instructions_at(10.into())
            .find_map(|location| match location {
                InstructionLocation::Terminator { block } => Some(block),
                InstructionLocation::BlockParameter { .. }
                | InstructionLocation::Operation { .. } => {
                    None
                }
            })
    );
    assert!(!terminator.uses().contains(&match_value));
    assert_eq!(
        ir.source_map().origin_of(InstructionLocation::Terminator {
            block: ir.entry_block(),
        }),
        Some(ProgramCounter::from(1))
    );
}

#[test]
fn fallible_exit_keeps_normal_then_ordered_handler_arms() {
    let exception_table = vec![
        ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: Some("java/lang/RuntimeException".parse().unwrap()),
        },
        ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 20.into(),
            catch_type: Some("java/lang/Throwable".parse().unwrap()),
        },
    ];
    let method = method(
        [
            (0, Instruction::AConstNull),
            (
                1,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2, Instruction::Pop),
            (3, Instruction::Return),
            (10, Instruction::AStore0),
            (11, Instruction::Return),
            (20, Instruction::AStore0),
            (21, Instruction::Return),
        ],
        "()V",
        exception_table,
    );
    let ir = build(&method).unwrap();
    let fallible = &ir.block(ir.entry_block()).unwrap().terminator;

    assert!(matches!(fallible, Terminator::Try { .. }));
    let entry_loc = InstructionLocation::Terminator {
        block: ir.entry_block(),
    };
    assert_eq!(ir.source_map().origin_of(entry_loc), Some(1.into()));
    assert!(matches!(
        fallible.successors().next().unwrap().transfer(),
        Some(ControlTransfer::Unconditional)
    ));
    let handler_types = fallible
        .successors()
        .skip(1)
        .map(|successor| match successor.transfer() {
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
    let method = method(
        [(0, Instruction::ALoad0), (1, Instruction::AThrow)],
        "(Ljava/lang/Throwable;)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let block = ir.block(ir.entry_block()).unwrap();

    assert!(matches!(block.terminator, Terminator::Throw { .. }));
    let entry_loc = InstructionLocation::Terminator {
        block: ir.entry_block(),
    };
    assert_eq!(
        ir.source_map().origin_of(entry_loc),
        Some(ProgramCounter::from(1))
    );
}
