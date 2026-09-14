use super::*;

#[test]
fn catch_all_preserves_precedence_and_shadows_later_handlers() {
    let runtime_exception: ClassRef = "java/lang/RuntimeException".parse().unwrap();
    let exception: ClassRef = "java/lang/Exception".parse().unwrap();
    let method = method(
        [
            (0, Instruction::ALoad0),
            (
                1,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
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
    let fallible = block_containing_instruction(&ir, instruction_at(&ir, 1.into()).id());
    let transfers = fallible
        .terminator()
        .successors()
        .iter()
        .map(Successor::transfer)
        .collect::<Vec<_>>();

    assert_eq!(transfers.len(), 3);
    assert!(matches!(transfers[0], ControlTransfer::Normal));
    assert!(
        matches!(transfers[1], ControlTransfer::Exception(Some(caught)) if caught == &runtime_exception)
    );
    assert!(matches!(transfers[2], ControlTransfer::Exception(None)));
    assert_eq!(ir.source_map().instructions_at(30.into()).count(), 0);
    assert!(
        !transfers
            .iter()
            .any(|transfer| matches!(transfer, ControlTransfer::Unwind))
    );
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
    assert!(ir.blocks().all(|block| {
        !matches!(block.terminator().kind(), TerminatorKind::Fallible)
            && block.terminator().successors().iter().all(|successor| {
                matches!(successor.transfer(), ControlTransfer::Unconditional)
                    || matches!(successor.transfer(), ControlTransfer::Conditional(_))
            })
    }));
    assert_eq!(ir.source_map().instructions_at(10.into()).count(), 0);
}
