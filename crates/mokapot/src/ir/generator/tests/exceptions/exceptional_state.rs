use crate::ir::BlockKind;

use super::*;

#[test]
fn exceptional_landing_splits_normal_and_exceptional_states_at_one_pc() {
    let str_type = "java/lang/String".parse().unwrap();
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::CheckCast(str_type)),
        (2, Instruction::AStore1),
        (3, Instruction::Return),
    ];
    let table = vec![handler(1.into()..2.into(), 2.into(), None)];
    let method = method(body, "(Ljava/lang/Object;)V", table);
    let ir = build(&method).unwrap();
    let location = ir.source_map().instructions_at(1.into()).next().unwrap();
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

    assert_ne!(normal_target, pad_id);
    let pad = ir.block(pad_id).unwrap();
    let BlockKind::LandingPad { exception: caught } = pad.kind else {
        panic!("exceptional successor must target a landing pad");
    };
    assert_eq!(
        ir.definition_of(caught),
        Some(ValueDefinition::CaughtException(pad_id))
    );
    assert!(pad.parameters.is_empty());
    assert!(pad.operations.is_empty());
    assert_eq!(pad.terminator.successors().count(), 1);
    let successor = pad.terminator.successors().next().unwrap();
    assert!(matches!(
        successor.transfer(),
        Some(&ControlTransfer::Unconditional)
    ));
    assert_eq!(successor.block_target(), Some(normal_target));
    let loc = InstructionLocation::Terminator { block: pad_id };
    assert_eq!(ir.source_map().origin_of(loc), None);
}

#[test]
fn exceptional_state_excludes_the_fallible_result() {
    let str_type = "java/lang/String".parse().unwrap();
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::CheckCast(str_type)),
        (2, Instruction::AReturn),
        (10, Instruction::AStore1),
        (11, Instruction::ALoad0),
        (12, Instruction::AReturn),
    ];
    let table = vec![handler(1.into()..2.into(), 10.into(), None)];
    let method = method(body, "(Ljava/lang/Object;)Ljava/lang/Object;", table);
    let ir = build(&method).unwrap();
    let result = ir
        .source_map()
        .instructions_at(1.into())
        .find_map(|it| match ir.instruction(it) {
            Some(InstructionRef::Terminator(terminator)) => terminator.def(),
            Some(InstructionRef::BlockParameter(_) | InstructionRef::Operation(_)) | None => None,
        })
        .expect("checkcast must define a result");
    let location = ir.source_map().instructions_at(1.into()).next().unwrap();
    assert_eq!(
        ir.definition_of(result),
        Some(ValueDefinition::Instruction(location))
    );
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
    let pad = ir
        .block(target(|it| matches!(it, ControlTransfer::Exception(None))))
        .unwrap();
    let handler_id = pad
        .terminator
        .successors()
        .next()
        .unwrap()
        .block_target()
        .unwrap();
    let handler = ir.block(handler_id).unwrap();

    assert!(matches!(
        ir.block(normal_target).unwrap().terminator,
        Terminator::TryReturn { value: Some(value), .. } if value == result
    ));
    assert!(matches!(
        handler.terminator,
        Terminator::TryReturn { value: Some(value), .. }
            if value == ir.parameter_values()[0] && value != result
    ));
}

#[test]
fn exception_table_arms_share_one_handler_entry_at_the_same_pc() {
    let runtime: ClassRef = "java/lang/RuntimeException".parse().unwrap();
    let str_type = "java/lang/String".parse().unwrap();
    let body = [
        (0, Instruction::AConstNull),
        (1, Instruction::CheckCast(str_type)),
        (2, Instruction::Return),
        (10, Instruction::AStore0),
        (11, Instruction::Return),
    ];
    let table = vec![
        handler(1.into()..2.into(), 10.into(), Some(runtime.clone())),
        handler(1.into()..2.into(), 10.into(), None),
    ];
    let method = method(body, "()V", table);
    let ir = build(&method).unwrap();
    let location = ir.source_map().instructions_at(1.into()).next().unwrap();
    let fallible = block_containing_instruction(&ir, location);
    let exceptional = fallible
        .terminator
        .successors()
        .filter(|it| matches!(it.transfer(), Some(ControlTransfer::Exception(_))))
        .collect::<Vec<_>>();

    assert_eq!(exceptional.len(), 2);
    assert_eq!(exceptional[0].block_target(), exceptional[1].block_target());
    assert!(matches!(
        exceptional[0].transfer(),
        Some(ControlTransfer::Exception(Some(caught))) if caught == &runtime
    ));
    assert!(matches!(
        exceptional[1].transfer(),
        Some(ControlTransfer::Exception(None))
    ));
    let pad = ir.block(exceptional[0].block_target().unwrap()).unwrap();
    assert!(matches!(pad.kind, BlockKind::LandingPad { .. }));
    let blocks = reachable_blocks(&ir);
    let pads = blocks
        .iter()
        .filter(|(_, it)| matches!(it.kind, BlockKind::LandingPad { .. }));
    assert_eq!(pads.count(), 1);
}
