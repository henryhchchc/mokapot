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
        let location = ir
            .source_map()
            .instructions_at(pc.into())
            .find(|&loc| {
                ir.instruction(loc)
                    .is_some_and(|it| matches!(it, InstructionRef::Operation(_)))
            })
            .unwrap();
        let block = block_containing_instruction(&ir, location);
        block
            .terminator
            .successors()
            .iter()
            .find(|successor| matches!(successor.transfer(), ControlTransfer::Unwind))
            .unwrap()
            .target()
    });

    assert_eq!(unwind_targets[0], unwind_targets[1]);
    let unwind = ir.block(unwind_targets[0]).unwrap();
    assert_eq!(unwind.terminator.kind(), &TerminatorKind::Unwind);
    assert!(unwind.phis.is_empty());
    assert!(unwind.operations.is_empty());
    assert!(unwind.terminator.successors().is_empty());
    let terminator_loc = InstructionLocation::Terminator {
        block: unwind_targets[0],
    };
    assert_eq!(ir.source_map().origin_of(terminator_loc), None);

    let mut edge_ids = HashSet::new();
    for (_, block) in ir.blocks() {
        for successor in block.terminator.successors() {
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
