use super::*;

#[test]
fn source_map_is_fixed_before_canonicalization() {
    let method = method(
        [(0, Instruction::IConst0), (1, Instruction::IReturn)],
        "()I",
        vec![],
    );
    let cfg = crate::ir::generator::bytecode_cfg::build(&method).unwrap();
    let mut draft = crate::ir::generator::bytecode_analysis::analyze(&cfg).unwrap();

    let operation = draft
        .source_map
        .instructions_at(0.into())
        .next()
        .expect("analysis must map the constant definition");
    let terminator = draft
        .source_map
        .instructions_at(1.into())
        .next()
        .expect("analysis must map the fallible return");
    assert!(matches!(operation, InstructionLocation::Operation { .. }));
    assert!(matches!(terminator, InstructionLocation::Terminator { .. }));
    let expected = draft.source_map.mappings().collect::<Vec<_>>();

    crate::ir::generator::canonicalize::canonicalize(&mut draft).unwrap();
    assert_eq!(draft.source_map.mappings().collect::<Vec<_>>(), expected);

    let ir = crate::ir::generator::finish::finish(&method, draft).unwrap();
    assert_eq!(ir.source_map().mappings().collect::<Vec<_>>(), expected);
}

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
    let block = ir.block(ir.entry_block()).unwrap();

    assert_eq!(ir.blocks().len(), 1);
    assert_eq!(block.operations.len(), 1);
    assert!(matches!(
        block.terminator,
        Terminator::TryReturn { value: Some(_), .. }
    ));

    let definition = ir
        .source_map()
        .instructions_at(0.into())
        .next()
        .expect("the constant definition must retain its source");
    assert!(matches!(
        ir.instruction(definition),
        Some(InstructionRef::Operation(_))
    ));
    let large_block_id = InstructionLocation::Operation {
        block: BlockId::new(u32::MAX),
        index: 0,
    };
    assert!(ir.instruction(large_block_id).is_none());
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
fn unreachable_frame_invalid_bytecode_is_omitted() {
    let method = method(
        [
            (0, Instruction::Goto(10.into())),
            (3, Instruction::IAdd),
            (10, Instruction::Return),
        ],
        "()V",
        vec![],
    );

    let ir = build(&method).expect("unreachable instructions must not contribute frame facts");
    assert_eq!(ir.source_map().instructions_at(3.into()).count(), 0);
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
    let (loop_block_id, loop_block) = ir
        .blocks()
        .find(|(block_id, block)| {
            block
                .terminator
                .successors()
                .any(|successor| successor.block_target() == Some(*block_id))
        })
        .expect("the backward target must form a self-loop");
    assert_eq!(
        loop_block
            .terminator
            .successors()
            .next()
            .unwrap()
            .block_target(),
        Some(loop_block_id)
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
    let fallthrough = entry
        .terminator
        .successors()
        .nth(1)
        .unwrap()
        .block_target()
        .unwrap();
    let synthetic = &ir.block(fallthrough).unwrap().terminator;

    assert!(matches!(synthetic, Terminator::Goto { .. }));
    assert_eq!(ir.source_map().instructions_at(2.into()).count(), 0);
    assert!(matches!(synthetic, Terminator::Goto { .. }));
    assert_eq!(
        ir.source_map()
            .origin_of(InstructionLocation::Terminator { block: fallthrough }),
        None
    );
}
