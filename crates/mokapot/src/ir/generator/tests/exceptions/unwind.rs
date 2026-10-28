use super::*;

#[test]
fn unhandled_exceptions_target_the_method_unwind_exit() {
    let mut pool = ConstantPool::new();
    let string_type = pool.insert_reference_type(ref_t("java/lang/String"));
    let integer_type = pool.insert_reference_type(ref_t("java/lang/Integer"));
    let body = [
        (0, InstructionData::ALoad0),
        (1, InstructionData::CheckCast(string_type)),
        (2, InstructionData::Pop),
        (3, InstructionData::ALoad0),
        (4, InstructionData::CheckCast(integer_type)),
        (5, InstructionData::AReturn),
    ];
    let ir = lift_with_pool(pool, body, "(Ljava/lang/Object;)Ljava/lang/Object;", vec![]);
    for pc in [1, 4] {
        assert_matches!(
            terminator_at(&ir, pc.into()),
            Terminator::Try { exceptional, .. } if exceptional == &[Successor::Unwind]
        );
    }
    let blocks = ir.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 3);
    assert!(blocks.iter().any(|(id, _)| *id == ir.entry.block));
    for (id, block) in blocks {
        assert_eq!(ir.block(id), Some(block));
    }
}

#[test]
fn throw_has_only_ordered_exceptional_outcomes() {
    let runtime = cls_name("java/lang/RuntimeException");
    let body = [
        (0, InstructionData::ALoad0),
        (1, InstructionData::AThrow),
        (10, InstructionData::AStore1),
        (11, InstructionData::Return),
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
