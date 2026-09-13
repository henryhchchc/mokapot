use super::*;

#[test]
fn array_write_is_an_effect_without_a_definition() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::ILoad1),
            (2, Instruction::ILoad2),
            (3, Instruction::IAStore),
            (4, Instruction::Return),
        ],
        "([III)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let effect = ir
        .source_map()
        .instructions_at(3.into())
        .find_map(|id| {
            ir.blocks()
                .flat_map(BasicBlock::operations)
                .find(|instruction| instruction.id() == id)
        })
        .unwrap();

    assert!(matches!(effect.kind(), OperationKind::Effect { .. }));
    assert_eq!(effect.def(), None);
    assert_eq!(effect.uses().len(), 3);
    assert_eq!(ir.source_map().instructions_at(0.into()).count(), 0);
    assert_eq!(ir.source_map().instructions_at(1.into()).count(), 0);
    assert_eq!(ir.source_map().instructions_at(2.into()).count(), 0);
}

#[test]
fn monitor_operations_are_effects_without_definitions() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::MonitorEnter),
            (2, Instruction::ALoad0),
            (3, Instruction::MonitorExit),
            (4, Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let instructions = ir
        .blocks()
        .flat_map(BasicBlock::operations)
        .collect::<Vec<_>>();

    assert_eq!(instructions.len(), 2);
    assert!(instructions.iter().all(|instruction| {
        instruction.def().is_none() && matches!(instruction.kind(), OperationKind::Effect { .. })
    }));
}
