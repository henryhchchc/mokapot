use super::*;

#[test]
fn catch_all_preserves_precedence_and_shadows_later_handlers() {
    let runtime_exception: ClassRef = "java/lang/RuntimeException".parse().unwrap();
    let exception: ClassRef = "java/lang/Exception".parse().unwrap();
    let str_type = "java/lang/String".parse().unwrap();
    let method = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::CheckCast(str_type)),
            (2, Instruction::Pop),
            (3, Instruction::Return),
            (10, Instruction::AStore1),
            (11, Instruction::Return),
            (20, Instruction::AStore1),
            (21, Instruction::Return),
            (30, Instruction::AStore1),
            (31, Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![
            ExceptionTableEntry {
                covered_pc: 1.into()..2.into(),
                handler_pc: 10.into(),
                catch_type: Some(runtime_exception.clone()),
            },
            ExceptionTableEntry {
                covered_pc: 1.into()..2.into(),
                handler_pc: 20.into(),
                catch_type: None,
            },
            ExceptionTableEntry {
                covered_pc: 1.into()..2.into(),
                handler_pc: 30.into(),
                catch_type: Some(exception),
            },
        ],
    );
    let ir = build(&method).unwrap();
    let fallible_location = ir.source_map().instructions_at(1.into()).next().unwrap();
    let fallible = block_containing_instruction(&ir, fallible_location);
    let transfers = fallible
        .terminator
        .successors()
        .map(Successor::transfer)
        .collect::<Vec<_>>();

    assert_eq!(transfers.len(), 3);
    assert!(matches!(transfers[0], Some(ControlTransfer::Unconditional)));
    assert!(
        matches!(transfers[1], Some(ControlTransfer::Exception(Some(caught))) if caught == &runtime_exception)
    );
    assert!(matches!(
        transfers[2],
        Some(ControlTransfer::Exception(None))
    ));
    assert_eq!(ir.source_map().instructions_at(30.into()).count(), 0);
    assert!(!transfers.iter().any(Option::is_none));
}

#[test]
fn protected_nonthrowing_operations_do_not_reach_a_handler_or_unwind() {
    let method = method(
        [
            (0, Instruction::IConst0),
            (1, Instruction::Pop),
            (2, Instruction::Return),
            (10, Instruction::AStore0),
            (11, Instruction::Return),
        ],
        "()V",
        vec![ExceptionTableEntry {
            covered_pc: 0.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: None,
        }],
    );
    let ir = build(&method).unwrap();

    assert_eq!(ir.blocks().len(), 1);
    let entry = ir.block(ir.entry_block()).unwrap();
    assert!(matches!(
        entry.terminator,
        Terminator::TryReturn { value: None, .. }
    ));
    assert_eq!(entry.terminator.successors().count(), 1);
    assert_eq!(
        entry.terminator.successors().next().unwrap().transfer(),
        None
    );
    assert_eq!(ir.source_map().instructions_at(10.into()).count(), 0);
}
