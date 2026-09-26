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
    let unwind_targets = [1, 4].map(|it| {
        let location = ir
            .source_map
            .instructions_at(it.into())
            .find(|&it| {
                ir.instruction(it)
                    .is_some_and(|it| matches!(it, InstructionRef::Terminator(_)))
            })
            .unwrap();
        let block = block_containing_instruction(&ir, location);
        block
            .terminator
            .successors()
            .find(|it| it.block_target().is_none())
            .unwrap()
            .block_target()
    });

    assert_eq!(unwind_targets, [None; 2]);
    assert_eq!(reachable_blocks(&ir).len(), 3);
}

#[test]
fn throw_has_only_ordered_exceptional_outcomes() {
    let runtime = cls_r("java/lang/RuntimeException");
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::AThrow),
        (10, Instruction::AStore1),
        (11, Instruction::Return),
    ];
    let table = vec![handler(1.into()..2.into(), 10.into(), Some(runtime))];
    let ir = lift(body, "(Ljava/lang/Throwable;)V", table);
    let throw = terminator_at(&ir, 1.into());
    let transfers = throw
        .successors()
        .map(Successor::transfer)
        .collect::<Vec<_>>();

    assert!(matches!(throw, Terminator::Throw { .. }));
    assert_eq!(transfers.len(), 2);
    assert!(matches!(
        transfers[0],
        Some(ControlTransfer::Exception(Some(_)))
    ));
    assert_eq!(transfers[1], None);
    assert!(
        !transfers
            .iter()
            .any(|it| matches!(it, Some(ControlTransfer::Unconditional)))
    );
}
