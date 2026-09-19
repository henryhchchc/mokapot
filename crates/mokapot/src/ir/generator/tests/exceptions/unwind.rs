use super::*;
use crate::ir::SuccessorTarget;

#[test]
fn unhandled_exceptions_target_the_method_unwind_exit() {
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
            .find(|successor| matches!(successor.transfer(), ControlTransfer::Unwind))
            .unwrap()
            .target()
    });

    assert_eq!(unwind_targets, [SuccessorTarget::Unwind; 2]);
    assert_eq!(ir.blocks().len(), 3);

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
        .map(Successor::transfer)
        .collect::<Vec<_>>();

    assert!(matches!(throw, Terminator::Throw { .. }));
    assert_eq!(transfers.len(), 2);
    assert!(matches!(transfers[0], ControlTransfer::Exception(Some(_))));
    assert!(matches!(transfers[1], ControlTransfer::Unwind));
    assert!(
        !transfers
            .iter()
            .any(|transfer| matches!(transfer, ControlTransfer::Unconditional))
    );
}
