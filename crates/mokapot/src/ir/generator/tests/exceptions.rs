#[allow(
    clippy::wildcard_imports,
    reason = "generator tests share the fixture helpers from their parent module"
)]
use super::*;
use crate::{ir::DefUseChain, jvm::references::ClassRef};

fn instruction_at(method: &MokaIRMethod, pc: ProgramCounter) -> &MokaInstruction {
    method
        .source_map()
        .instructions_at(pc)
        .find_map(|id| {
            method
                .blocks()
                .flat_map(BasicBlock::instructions)
                .find(|instruction| instruction.id() == id)
        })
        .expect("the source PC must map to an ordinary instruction")
}

fn terminator_at(method: &MokaIRMethod, pc: ProgramCounter) -> &Terminator {
    method
        .source_map()
        .instructions_at(pc)
        .find_map(|id| {
            method
                .blocks()
                .map(BasicBlock::terminator)
                .find(|terminator| terminator.id() == id)
        })
        .expect("the source PC must map to a terminator")
}

fn block_containing_instruction(method: &MokaIRMethod, instruction: InstructionId) -> &BasicBlock {
    method
        .blocks()
        .find(|block| {
            block
                .instructions()
                .iter()
                .any(|candidate| candidate.id() == instruction)
        })
        .expect("the instruction must belong to a block")
}

#[test]
fn exceptional_landing_splits_normal_and_exceptional_states_at_one_pc() {
    let method = method(
        [
            (0.into(), Instruction::ALoad0),
            (
                1.into(),
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2.into(), Instruction::AStore1),
            (3.into(), Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 2.into(),
            catch_type: None,
        }],
    );
    let ir = method.brew().unwrap();
    let fallible = block_containing_instruction(&ir, instruction_at(&ir, 1.into()).id());

    let normal_target = fallible
        .terminator()
        .successors()
        .iter()
        .find(|successor| matches!(successor.transfer(), ControlTransfer::Normal))
        .unwrap()
        .target();
    let handler_entry = fallible
        .terminator()
        .successors()
        .iter()
        .find(|successor| matches!(successor.transfer(), ControlTransfer::Exception(None)))
        .unwrap()
        .target();

    assert_ne!(normal_target, handler_entry);
    let handler_entry = ir.block(handler_entry).unwrap();
    let caught = ir.caught_exception(handler_entry.id()).unwrap();
    assert_eq!(
        ir.value_definition(caught),
        Some(ValueDefinition::CaughtException(handler_entry.id()))
    );
    assert!(handler_entry.phis().is_empty());
    assert!(handler_entry.instructions().is_empty());
    assert_eq!(handler_entry.terminator().successors().len(), 1);
    assert!(matches!(
        handler_entry.terminator().successors()[0].transfer(),
        ControlTransfer::Unconditional
    ));
    assert_eq!(
        handler_entry.terminator().successors()[0].target(),
        normal_target
    );
    assert_eq!(
        ir.source_map()
            .origins_of(handler_entry.terminator().id())
            .count(),
        0
    );
}

#[test]
fn catch_all_preserves_precedence_and_shadows_later_handlers() {
    let runtime_exception: ClassRef = "java/lang/RuntimeException".parse().unwrap();
    let exception: ClassRef = "java/lang/Exception".parse().unwrap();
    let method = method(
        [
            (0.into(), Instruction::ALoad0),
            (
                1.into(),
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2.into(), Instruction::Pop),
            (3.into(), Instruction::Return),
            (10.into(), Instruction::AStore1),
            (11.into(), Instruction::Return),
            (20.into(), Instruction::AStore1),
            (21.into(), Instruction::Return),
            (30.into(), Instruction::AStore1),
            (31.into(), Instruction::Return),
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
    let ir = method.brew().unwrap();
    let fallible = block_containing_instruction(&ir, instruction_at(&ir, 1.into()).id());
    let transfers = fallible
        .terminator()
        .successors()
        .iter()
        .map(Successor::transfer)
        .collect::<Vec<_>>();

    assert_eq!(transfers.len(), 3);
    assert!(matches!(transfers[0], ControlTransfer::Normal));
    assert!(matches!(
        transfers[1],
        ControlTransfer::Exception(Some(caught)) if caught == &runtime_exception
    ));
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
            (0.into(), Instruction::IConst0),
            (1.into(), Instruction::Pop),
            (2.into(), Instruction::Return),
            (10.into(), Instruction::AStore0),
            (11.into(), Instruction::Return),
        ],
        "()V",
        vec![ExceptionTableEntry {
            covered_pc: 0.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: None,
        }],
    );
    let ir = method.brew().unwrap();

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

#[test]
fn exceptional_state_excludes_the_fallible_result() {
    let method = method(
        [
            (0.into(), Instruction::ALoad0),
            (
                1.into(),
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2.into(), Instruction::AReturn),
            (10.into(), Instruction::AStore1),
            (11.into(), Instruction::ALoad0),
            (12.into(), Instruction::AReturn),
        ],
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: None,
        }],
    );
    let ir = method.brew().unwrap();
    let result = instruction_at(&ir, 1.into()).def().unwrap();
    let normal_return = terminator_at(&ir, 2.into()).id();
    let handler_return = terminator_at(&ir, 12.into()).id();
    let uses = DefUseChain::new(&ir).used_at(result);

    assert_eq!(uses, BTreeSet::from([normal_return]));
    assert!(!uses.contains(&handler_return));
    assert!(
        ir.blocks()
            .filter_map(|block| ir.caught_exception(block.id()))
            .all(|caught| ir.value_definition(caught).is_some())
    );
}

#[test]
fn unhandled_exceptions_share_one_synthetic_unwind_block() {
    let method = method(
        [
            (0.into(), Instruction::ALoad0),
            (
                1.into(),
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2.into(), Instruction::Pop),
            (3.into(), Instruction::ALoad0),
            (
                4.into(),
                Instruction::CheckCast("java/lang/Integer".parse().unwrap()),
            ),
            (5.into(), Instruction::AReturn),
        ],
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        vec![],
    );
    let ir = method.brew().unwrap();
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
    assert!(unwind.instructions().is_empty());
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
        for instruction in block.instructions() {
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
            (0.into(), Instruction::ALoad0),
            (1.into(), Instruction::AThrow),
            (10.into(), Instruction::AStore1),
            (11.into(), Instruction::Return),
        ],
        "(Ljava/lang/Throwable;)V",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: Some("java/lang/RuntimeException".parse().unwrap()),
        }],
    );
    let ir = method.brew().unwrap();
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
            .any(|transfer| matches!(transfer, ControlTransfer::Normal))
    );
}
