use super::*;

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
    let ir = build(&method).unwrap();
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
        ir.definition_of(caught),
        Some(ValueDefinition::CaughtException(handler_entry.id()))
    );
    assert!(handler_entry.phis().is_empty());
    assert!(handler_entry.operations().is_empty());
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
    let ir = build(&method).unwrap();
    let result = instruction_at(&ir, 1.into()).def().unwrap();
    let normal_return = terminator_at(&ir, 2.into()).id();
    let handler_return = terminator_at(&ir, 12.into()).id();
    let uses = DefUseChain::new(&ir)
        .uses_of(result)
        .collect::<BTreeSet<_>>();

    assert_eq!(uses, BTreeSet::from([UseSite::Instruction(normal_return)]));
    assert!(!uses.contains(&UseSite::Instruction(handler_return)));
    assert!(
        ir.blocks()
            .filter_map(|block| ir.caught_exception(block.id()))
            .all(|caught| ir.definition_of(caught).is_some())
    );
}
