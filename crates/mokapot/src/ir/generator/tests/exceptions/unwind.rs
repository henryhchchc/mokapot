use super::*;

#[test]
fn unhandled_exceptions_target_the_method_unwind_exit() {
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::CheckCast(ref_t("java/lang/String"))),
        (2, Instruction::Pop),
        (3, Instruction::ALoad0),
        (4, Instruction::CheckCast(ref_t("java/lang/Integer"))),
        (5, Instruction::AReturn),
    ];
    let ir = lift(body, "(Ljava/lang/Object;)Ljava/lang/Object;", vec![]);
    for pc in [1, 4] {
        assert_matches!(
            terminator_at(&ir, pc.into()),
            Terminator::Try { exceptional, .. } if exceptional == &[Successor::Unwind]
        );
    }
    assert_eq!(reachable_blocks(&ir).len(), 3);
}

#[test]
fn throw_has_only_ordered_exceptional_outcomes() {
    let runtime = cls_name("java/lang/RuntimeException");
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::AThrow),
        (10, Instruction::AStore1),
        (11, Instruction::Return),
    ];
    let table = vec![handler(
        1.into()..2.into(),
        10.into(),
        Some(runtime.clone()),
    )];
    let ir = lift(body, "(Ljava/lang/Throwable;)V", table);
    let throw = terminator_at(&ir, 1.into());
    let transfers = throw
        .successors()
        .map(Successor::transfer)
        .collect::<Vec<_>>();

    assert_matches!(throw, Terminator::Throw { value, .. } if *value == ir.parameters[0]);
    assert_eq!(entry_origin(&ir), Some(1.into()));
    let caught = ControlTransfer::Exception(Some(runtime));
    assert_eq!(transfers, [Some(&caught), None]);
}
