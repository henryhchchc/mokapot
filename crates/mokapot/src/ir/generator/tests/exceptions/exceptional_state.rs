use super::*;

#[test]
fn exceptional_landing_splits_normal_and_exceptional_states_at_one_pc() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (
                1,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
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
        .iter()
        .find(|successor| matches!(successor.transfer(), ControlTransfer::Unconditional))
        .unwrap()
        .target();
    let handler_entry_id = fallible
        .terminator
        .successors()
        .iter()
        .find(|successor| matches!(successor.transfer(), ControlTransfer::Exception(None)))
        .unwrap()
        .target();

    assert_ne!(normal_target, handler_entry_id);
    let handler_entry = ir.block(handler_entry_id).unwrap();
    let caught = ir.caught_exception(handler_entry_id).unwrap();
    assert_eq!(
        ir.definition_of(caught),
        Some(ValueDefinition::CaughtException(handler_entry_id))
    );
    assert!(handler_entry.phis.is_empty());
    assert!(handler_entry.operations.is_empty());
    assert_eq!(handler_entry.terminator.successors().len(), 1);
    assert!(matches!(
        handler_entry.terminator.successors()[0].transfer(),
        ControlTransfer::Unconditional
    ));
    assert_eq!(
        handler_entry.terminator.successors()[0].target(),
        normal_target
    );
    let loc = InstructionLocation::Terminator {
        block: handler_entry_id,
    };
    assert_eq!(ir.source_map().origin_of(loc), None);
}

#[test]
fn exceptional_state_excludes_the_fallible_result() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (
                1,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2, Instruction::AReturn),
            (10, Instruction::AStore1),
            (11, Instruction::ALoad0),
            (12, Instruction::AReturn),
        ],
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: None,
        }],
    );
    let ir = build(&method).unwrap();
    assert!(
        ir.blocks()
            .filter_map(|(block_id, _)| ir.caught_exception(block_id))
            .all(|caught| ir.definition_of(caught).is_some())
    );
}

#[test]
fn exception_table_arms_share_one_handler_entry_at_the_same_pc() {
    let runtime_exception: crate::jvm::references::ClassRef =
        "java/lang/RuntimeException".parse().unwrap();
    let method = method(
        [
            (0, Instruction::AConstNull),
            (
                1,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2, Instruction::Return),
            (10, Instruction::AStore0),
            (11, Instruction::Return),
        ],
        "()V",
        vec![
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
        ],
    );
    let ir = build(&method).unwrap();
    let fallible_location = ir.source_map().instructions_at(1.into()).next().unwrap();
    let fallible = block_containing_instruction(&ir, fallible_location);
    let exceptional = fallible
        .terminator
        .successors()
        .iter()
        .filter(|successor| matches!(successor.transfer(), ControlTransfer::Exception(_)))
        .collect::<Vec<_>>();

    assert_eq!(exceptional.len(), 2);
    assert_eq!(exceptional[0].target(), exceptional[1].target());
    assert!(matches!(
        exceptional[0].transfer(),
        ControlTransfer::Exception(Some(caught)) if caught == &runtime_exception
    ));
    assert!(matches!(
        exceptional[1].transfer(),
        ControlTransfer::Exception(None)
    ));
    let handler = ir.block(exceptional[0].target()).unwrap();
    assert!(handler.caught_exception.is_some());
    assert_eq!(
        ir.blocks()
            .filter(|(_, block)| block.caught_exception.is_some())
            .count(),
        1
    );
}
