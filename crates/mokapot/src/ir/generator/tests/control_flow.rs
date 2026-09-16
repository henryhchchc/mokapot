use super::*;
use crate::{
    ir::{
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, PathValue},
        },
        expression::Condition,
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
    let switch = ir.block(ir.entry_block()).unwrap().terminator();

    assert!(matches!(switch.kind(), TerminatorKind::Switch { .. }));
    assert_eq!(switch.successors().len(), 3);
    assert_eq!(
        switch
            .successors()
            .iter()
            .map(Successor::id)
            .collect::<HashSet<_>>()
            .len(),
        3
    );
    assert!(
        switch
            .successors()
            .windows(2)
            .all(|pair| pair[0].target() == pair[1].target())
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
    let branch = ir.block(ir.entry_block()).unwrap().terminator();
    let match_value = ir.parameter_values()[0];

    assert_eq!(branch.kind(), &TerminatorKind::Branch);
    assert_eq!(branch.successors().len(), 2);
    assert_eq!(
        branch.successors()[0].transfer(),
        &ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(
            Condition::IsZero(PathValue::Variable(match_value)),
        )))
    );
    assert_eq!(
        branch.successors()[1].transfer(),
        &ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Negative(
            Condition::IsZero(PathValue::Variable(match_value)),
        )))
    );
    assert_eq!(
        ir.source_map().origins_of(branch.id()).collect::<Vec<_>>(),
        [ProgramCounter::from(1)]
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
    let branch = ir.block(ir.entry_block()).unwrap().terminator();
    let parameters = ir.parameter_values();

    assert_eq!(
        branch.successors()[0].transfer(),
        &ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(
            Condition::LessThan(
                PathValue::Variable(parameters[0]),
                PathValue::Variable(parameters[1]),
            ),
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
    let switch = ir.block(ir.entry_block()).unwrap().terminator();
    let match_value = ir.parameter_values()[0];
    let case_guard = |case| {
        ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(
            Condition::Equal(
                PathValue::Variable(match_value),
                PathValue::Constant(ConstantValue::Integer(case)),
            ),
        )))
    };

    assert_eq!(switch.kind(), &TerminatorKind::Switch { match_value });
    assert_eq!(switch.successors().len(), 3);
    assert!(
        switch
            .successors()
            .windows(2)
            .all(|pair| pair[0].target() == pair[1].target())
    );
    assert_eq!(switch.successors()[0].transfer(), &case_guard(3));
    assert_eq!(switch.successors()[1].transfer(), &case_guard(4));
    assert!(matches!(
        switch.successors()[2].transfer(),
        ControlTransfer::Conditional(guard) if guard.predicate_count() == 2
    ));
    assert_eq!(
        switch
            .successors()
            .iter()
            .map(Successor::id)
            .collect::<Vec<_>>(),
        (0..3).map(EdgeId::new).collect::<Vec<_>>()
    );
    assert_eq!(
        ir.source_map().origins_of(switch.id()).collect::<Vec<_>>(),
        [ProgramCounter::from(1)]
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
    let fallible = ir.block(ir.entry_block()).unwrap().terminator();

    assert_eq!(fallible.kind(), &TerminatorKind::Fallible);
    assert_eq!(ir.source_map().origins_of(fallible.id()).count(), 0);
    assert!(matches!(
        fallible.successors()[0].transfer(),
        ControlTransfer::Unconditional
    ));
    let handler_types = fallible.successors()[1..]
        .iter()
        .map(|successor| match successor.transfer() {
            ControlTransfer::Exception(Some(caught)) => caught.0.as_ref(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handler_types,
        ["java/lang/RuntimeException", "java/lang/Throwable"]
    );
}

#[test]
fn normally_reachable_handler_still_starts_a_block() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (
                1,
                Instruction::CheckCast("java/lang/Throwable".parse().unwrap()),
            ),
            (2, Instruction::AStore1),
            (3, Instruction::Return),
        ],
        "(Ljava/lang/Throwable;)V",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 2.into(),
            catch_type: Some("java/lang/Throwable".parse().unwrap()),
        }],
    );
    let ir = build(&method).unwrap();
    let entry = ir.block(ir.entry_block()).unwrap();
    let normal = entry
        .terminator()
        .successors()
        .iter()
        .find(|successor| matches!(successor.transfer(), ControlTransfer::Unconditional))
        .unwrap()
        .target();
    let handler = entry
        .terminator()
        .successors()
        .iter()
        .find(|successor| matches!(successor.transfer(), ControlTransfer::Exception(_)))
        .unwrap()
        .target();

    assert_eq!(ir.blocks().len(), 3);
    assert_ne!(handler, ir.entry_block());
    assert_ne!(handler, normal);
    assert_eq!(
        ir.block(handler).unwrap().terminator().successors()[0].target(),
        normal
    );
    assert_eq!(ir.block(handler).unwrap().operations().len(), 0);
    assert!(ir.caught_exception(handler).is_some());
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

    assert!(matches!(
        block.terminator().kind(),
        TerminatorKind::Throw(_)
    ));
    assert_eq!(
        ir.source_map()
            .origins_of(block.terminator().id())
            .collect::<Vec<_>>(),
        [ProgramCounter::from(1)]
    );
}
