use super::*;
use std::iter::once;

#[test]
fn straight_line_instructions_coalesce_into_one_block() {
    let method = method(
        [
            (0, Instruction::IConst0),
            (10, Instruction::IStore0),
            (20, Instruction::ILoad0),
            (30, Instruction::IReturn),
        ],
        "()I",
        vec![],
    );
    let ir = build(&method).unwrap();
    let blocks = ir.blocks().collect::<Vec<_>>();

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].operations().len(), 1);
    assert!(matches!(
        blocks[0].terminator().kind(),
        TerminatorKind::Return(Some(_))
    ));
    let ids = blocks[0]
        .operations()
        .iter()
        .map(Operation::id)
        .chain(once(blocks[0].terminator().id()))
        .collect::<HashSet<_>>();
    assert_eq!(ids.len(), 2);
}

#[test]
fn value_identities_do_not_depend_on_sparse_program_counters() {
    let compact = build(&method(
        [
            (0, Instruction::IConst0),
            (1, Instruction::Pop),
            (2, Instruction::IConst1),
            (3, Instruction::IReturn),
        ],
        "()I",
        vec![],
    ))
    .unwrap();
    let sparse = build(&method(
        [
            (0, Instruction::IConst0),
            (100, Instruction::Pop),
            (1000, Instruction::IConst1),
            (5000, Instruction::IReturn),
        ],
        "()I",
        vec![],
    ))
    .unwrap();
    let values = |method: &MokaIRMethod| {
        method
            .blocks()
            .flat_map(BasicBlock::operations)
            .filter_map(Operation::def)
            .collect::<Vec<_>>()
    };

    assert_eq!(values(&compact), values(&sparse));
}

#[test]
fn unreachable_bytecode_is_omitted() {
    let method = method(
        [
            (0, Instruction::Goto(100.into())),
            (10, Instruction::IConst0),
            (11, Instruction::IReturn),
            (100, Instruction::Return),
        ],
        "()V",
        vec![],
    );
    let ir = build(&method).unwrap();

    assert_eq!(ir.blocks().len(), 2);
    assert_eq!(ir.source_map().instructions_at(10.into()).count(), 0);
    assert_eq!(ir.source_map().instructions_at(11.into()).count(), 0);
}

#[test]
fn backward_target_starts_a_block_even_when_transfer_is_last() {
    let method = method(
        [
            (0, Instruction::Nop),
            (1, Instruction::Nop),
            (2, Instruction::Goto(1.into())),
        ],
        "()V",
        vec![],
    );
    let ir = build(&method).unwrap();

    assert_eq!(ir.blocks().len(), 2);
    let loop_block = ir.blocks().nth(1).unwrap();
    assert_eq!(
        loop_block.terminator().successors()[0].target(),
        loop_block.id()
    );
}

#[test]
fn diamond_has_an_unmapped_synthetic_fallthrough() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::IfEq(3.into())),
            (2, Instruction::Nop),
            (3, Instruction::Return),
        ],
        "(I)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let entry = ir.block(ir.entry_block()).unwrap();
    let fallthrough = entry.terminator().successors()[1].target();
    let synthetic = ir.block(fallthrough).unwrap().terminator();

    assert_eq!(synthetic.kind(), &TerminatorKind::Goto);
    assert_eq!(ir.source_map().instructions_at(2.into()).count(), 0);
    assert_eq!(ir.source_map().origins_of(synthetic.id()).count(), 0);
}
