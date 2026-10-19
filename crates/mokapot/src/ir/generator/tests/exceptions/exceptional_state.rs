use super::*;

#[test]
fn exceptional_state_excludes_the_fallible_result() {
    let mut pool = ConstantPool::new();
    let string_type = pool.insert_reference_type(ref_t("java/lang/String"));
    let body = [
        (0, InstructionData::ALoad0),
        (1, InstructionData::CheckCast(string_type)),
        (2, InstructionData::AReturn),
        (10, InstructionData::AStore1),
        (11, InstructionData::ALoad0),
        (12, InstructionData::AReturn),
    ];
    let table = vec![handler(1.into()..2.into(), 10.into(), None)];
    let ir = lift_with_pool(pool, body, "(Ljava/lang/Object;)Ljava/lang/Object;", table);
    let attempted = terminator_at(&ir, 1.into()).operation().unwrap();
    let Operation::Definition { value: result, .. } = attempted else {
        panic!("checkcast must define a result");
    };
    let result = *result;
    let location = ir.source_map.instructions_at(1.into()).next().unwrap();
    let fallible = block_containing_instruction(&ir, location);
    let target = |predicate: fn(&ControlTransfer) -> bool| {
        fallible
            .terminator
            .successors()
            .find(|it| it.transfer().is_some_and(predicate))
            .unwrap()
            .block_target()
            .unwrap()
    };
    let normal_target = target(|it| *it == ControlTransfer::Unconditional);
    let pad_id = target(|it| matches!(it, ControlTransfer::Exception(None)));
    let pad = block_of(&ir, pad_id);
    let handler = block_of(&ir, successor_target(pad, 0));

    assert!(matches!(
        block_of(&ir, normal_target).terminator,
        Terminator::Return { value: Some(value), .. } if value == result
    ));
    assert!(matches!(
        handler.terminator,
        Terminator::Return { value: Some(value), .. }
            if value == ir.parameters[0] && value != result
    ));
}
