use super::*;

#[test]
fn catch_all_preserves_precedence_and_shadows_later_handlers() {
    let runtime = cls_name("java/lang/RuntimeException");
    let exception = cls_name("java/lang/Exception");
    let mut pool = ConstantPool::new();
    let string_type = pool.insert_reference_type(ref_t("java/lang/String"));
    let body = [
        (0, InstructionData::ALoad0),
        (1, InstructionData::CheckCast(string_type)),
        (2, InstructionData::Pop),
        (3, InstructionData::Return),
        (10, InstructionData::AStore1),
        (11, InstructionData::Return),
        (20, InstructionData::AStore1),
        (21, InstructionData::Return),
        (30, InstructionData::AStore1),
        (31, InstructionData::Return),
    ];
    let table = vec![
        handler(1.into()..2.into(), 10.into(), Some(runtime.clone())),
        handler(1.into()..2.into(), 20.into(), None),
        handler(1.into()..2.into(), 30.into(), Some(exception)),
    ];
    let ir = lift_with_pool(pool, body, "(Ljava/lang/Object;)V", table);
    let location = ir.source_map.instructions_at(1.into()).next().unwrap();
    let fallible = block_containing_instruction(&ir, location);
    let transfers = fallible
        .terminator
        .successors()
        .map(Successor::transfer)
        .collect::<Vec<_>>();

    assert_eq!(transfers.len(), 3);
    assert!(matches!(transfers[0], Some(ControlTransfer::Unconditional)));
    assert!(matches!(transfers[1], Some(ControlTransfer::Exception(Some(it))) if it == &runtime));
    assert!(matches!(
        transfers[2],
        Some(ControlTransfer::Exception(None))
    ));
    assert_eq!(ir.source_map.instructions_at(30.into()).count(), 0);
    assert!(!transfers.iter().any(Option::is_none));
}

#[test]
fn protected_nonthrowing_operations_do_not_reach_a_handler_or_unwind() {
    let body = [
        (0, InstructionData::IConst0),
        (1, InstructionData::Pop),
        (2, InstructionData::Return),
        (10, InstructionData::AStore0),
        (11, InstructionData::Return),
    ];
    let table = vec![handler(0.into()..2.into(), 10.into(), None)];
    let ir = lift(body, "()V", table);
    let entry = &ir.block(ir.entry.block).unwrap().terminator;

    assert_eq!(reachable_blocks(&ir).len(), 1);
    assert!(matches!(entry, Terminator::Return { value: None, .. }));
    assert_eq!(entry.successors().count(), 1);
    assert_eq!(entry.successors().next().unwrap().transfer(), None);
    assert_eq!(ir.source_map.instructions_at(10.into()).count(), 0);
}
