use crate::ir::BlockKind;

use super::*;

#[test]
fn exceptional_landing_splits_normal_and_exceptional_states_at_one_pc() {
    let str_type = "java/lang/String".parse().unwrap();
    let method = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::CheckCast(str_type)),
            (2, Instruction::AStore1),
            (3, Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 2.into(),
            catch_type: None,
        }],
    );
    let ir = build(&method).unwrap();
    let fallible_location = ir.source_map().instructions_at(1.into()).next().unwrap();
    let fallible = block_containing_instruction(&ir, fallible_location);

    let normal_target = fallible
        .terminator
        .successors()
        .find(|successor| matches!(successor.transfer(), Some(ControlTransfer::Unconditional)))
        .unwrap()
        .block_target()
        .unwrap();
    let handler_bb_id = fallible
        .terminator
        .successors()
        .find(|successor| matches!(successor.transfer(), Some(ControlTransfer::Exception(None))))
        .unwrap()
        .block_target()
        .unwrap();

    assert_ne!(normal_target, handler_bb_id);
    let handler_bb = ir.block(handler_bb_id).unwrap();
    let BlockKind::LandingPad { exception: caught } = handler_bb.kind else {
        panic!("exceptional successor must target a landing pad");
    };
    assert_eq!(
        ir.definition_of(caught),
        Some(ValueDefinition::CaughtException(handler_bb_id))
    );
    assert!(handler_bb.parameters.is_empty());
    assert!(handler_bb.operations.is_empty());
    assert_eq!(handler_bb.terminator.successors().count(), 1);
    let successor = handler_bb.terminator.successors().next().unwrap();
    assert!(matches!(
        successor.transfer(),
        Some(&ControlTransfer::Unconditional)
    ));
    assert_eq!(successor.block_target(), Some(normal_target));
    let loc = InstructionLocation::Terminator {
        block: handler_bb_id,
    };
    assert_eq!(ir.source_map().origin_of(loc), None);
}

#[test]
fn exceptional_state_excludes_the_fallible_result() {
    let str_type = "java/lang/String".parse().unwrap();
    let instructions = [
        (0, Instruction::ALoad0),
        (1, Instruction::CheckCast(str_type)),
        (2, Instruction::AReturn),
        (10, Instruction::AStore1),
        (11, Instruction::ALoad0),
        (12, Instruction::AReturn),
    ];
    let exception_table = vec![ExceptionTableEntry {
        covered_pc: 1.into()..2.into(),
        handler_pc: 10.into(),
        catch_type: None,
    }];
    let method = method(
        instructions,
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        exception_table,
    );
    let ir = build(&method).unwrap();
    let fallible_result = ir
        .source_map()
        .instructions_at(1.into())
        .find_map(|location| match ir.instruction(location) {
            Some(InstructionRef::Terminator(terminator)) => terminator.def(),
            Some(InstructionRef::BlockParameter(_) | InstructionRef::Operation(_)) | None => None,
        })
        .expect("checkcast must define a result");
    let fallible_location = ir.source_map().instructions_at(1.into()).next().unwrap();
    assert_eq!(
        ir.definition_of(fallible_result),
        Some(ValueDefinition::Instruction(fallible_location))
    );
    let fallible = block_containing_instruction(&ir, fallible_location);
    let normal_target = fallible
        .terminator
        .successors()
        .find(|successor| matches!(successor.transfer(), Some(ControlTransfer::Unconditional)))
        .unwrap()
        .block_target()
        .unwrap();
    let handler_entry = fallible
        .terminator
        .successors()
        .find(|successor| matches!(successor.transfer(), Some(ControlTransfer::Exception(None))))
        .and_then(Successor::block_target)
        .and_then(|target| ir.block(target))
        .expect("the exceptional outcome must enter the handler");
    let handler_id = handler_entry
        .terminator
        .successors()
        .next()
        .unwrap()
        .block_target()
        .unwrap();
    let handler_bb = ir.block(handler_id).unwrap();

    assert!(matches!(
        ir.block(normal_target).unwrap().terminator,
        Terminator::TryReturn { value: Some(value), .. } if value == fallible_result
    ));
    assert!(matches!(
        handler_bb.terminator,
        Terminator::TryReturn { value: Some(value), .. }
            if value == ir.parameter_values()[0] && value != fallible_result
    ));
}

#[test]
fn exception_table_arms_share_one_handler_entry_at_the_same_pc() {
    let runtime_exception: crate::jvm::references::ClassRef =
        "java/lang/RuntimeException".parse().unwrap();
    let str_type = "java/lang/String".parse().unwrap();
    let instructions = [
        (0, Instruction::AConstNull),
        (1, Instruction::CheckCast(str_type)),
        (2, Instruction::Return),
        (10, Instruction::AStore0),
        (11, Instruction::Return),
    ];
    let exception_table = vec![
        ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: Some(runtime_exception.clone()),
        },
        ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: None,
        },
    ];
    let method = method(instructions, "()V", exception_table);
    let ir = build(&method).unwrap();
    let fallible_location = ir.source_map().instructions_at(1.into()).next().unwrap();
    let fallible = block_containing_instruction(&ir, fallible_location);
    let exceptional = fallible
        .terminator
        .successors()
        .filter(|successor| matches!(successor.transfer(), Some(ControlTransfer::Exception(_))))
        .collect::<Vec<_>>();

    assert_eq!(exceptional.len(), 2);
    assert_eq!(exceptional[0].block_target(), exceptional[1].block_target());
    assert!(matches!(
        exceptional[0].transfer(),
        Some(ControlTransfer::Exception(Some(caught))) if caught == &runtime_exception
    ));
    assert!(matches!(
        exceptional[1].transfer(),
        Some(ControlTransfer::Exception(None))
    ));
    let handler = ir.block(exceptional[0].block_target().unwrap()).unwrap();
    assert!(matches!(handler.kind, BlockKind::LandingPad { .. }));
    assert_eq!(
        ir.blocks()
            .filter(|(_, block)| matches!(block.kind, BlockKind::LandingPad { .. }))
            .count(),
        1
    );
}
