use super::*;

#[test]
fn unhandled_exceptions_share_one_synthetic_unwind_block() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (
                1,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2, Instruction::Pop),
            (3, Instruction::ALoad0),
            (
                4,
                Instruction::CheckCast("java/lang/Integer".parse().unwrap()),
            ),
            (5, Instruction::AReturn),
        ],
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![],
    );
    let ir = build(&method).unwrap();
    let unwind_targets = [1, 4].map(|pc| {
        let block = block_containing_instruction(&ir, instruction_at(&ir, pc.into()).id());
        block
            .terminator()
            .successors()
            .iter()
            .find(|successor| matches!(successor.transfer(), ControlTransfer::Unwind))
            .unwrap()
            .target()
    });

    assert_eq!(unwind_targets[0], unwind_targets[1]);
    let unwind = ir.block(unwind_targets[0]).unwrap();
    assert_eq!(unwind.terminator().kind(), &TerminatorKind::Unwind);
    assert!(unwind.phis().is_empty());
    assert!(unwind.operations().is_empty());
    assert!(unwind.terminator().successors().is_empty());
    assert_eq!(
        ir.source_map().origins_of(unwind.terminator().id()).count(),
        0
    );

    let mut instruction_ids = HashSet::new();
    let mut edge_ids = HashSet::new();
    for block in ir.blocks() {
        for phi in block.phis() {
            assert!(instruction_ids.insert(phi.id()));
        }
        for instruction in block.operations() {
            assert!(instruction_ids.insert(instruction.id()));
        }
        assert!(instruction_ids.insert(block.terminator().id()));
        for successor in block.terminator().successors() {
            assert!(edge_ids.insert(successor.id()));
        }
    }
}

#[test]
fn throw_has_only_ordered_exceptional_outcomes() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::AThrow),
            (10, Instruction::AStore1),
            (11, Instruction::Return),
        ],
        "(Ljava/lang/Throwable;)V",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: Some("java/lang/RuntimeException".parse().unwrap()),
        }],
    );
    let ir = build(&method).unwrap();
    let throw = terminator_at(&ir, 1.into());
    let transfers = throw
        .successors()
        .iter()
        .map(Successor::transfer)
        .collect::<Vec<_>>();

    assert!(matches!(throw.kind(), TerminatorKind::Throw(_)));
    assert_eq!(transfers.len(), 2);
    assert!(matches!(transfers[0], ControlTransfer::Exception(Some(_))));
    assert!(matches!(transfers[1], ControlTransfer::Unwind));
    assert!(
        !transfers
            .iter()
            .any(|transfer| matches!(transfer, ControlTransfer::Unconditional))
    );
}
