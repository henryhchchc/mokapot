use super::*;
use crate::ir::BlockId;
use std::iter::once;

/// Builds a method whose control flow needs a conditional branch and an
/// exception handler, and therefore several blocks.
fn branch_and_handler_method() -> Method {
    method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::IfNull(9.into())),
            (2, Instruction::AConstNull),
            (
                3,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (4, Instruction::Pop),
            (9, Instruction::Return),
            (10, Instruction::AStore0),
            (11, Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![ExceptionTableEntry {
            covered_pc: 3.into()..4.into(),
            handler_pc: 10.into(),
            catch_type: Some("java/lang/RuntimeException".parse().unwrap()),
        }],
    )
}

fn block_shape(ir: &MokaIRMethod) -> Vec<(BlockId, &TerminatorKind)> {
    ir.blocks()
        .map(|block| (block.id(), block.terminator().kind()))
        .collect()
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

#[test]
fn block_identities_are_dense_and_ascending() {
    let ir = build(&branch_and_handler_method()).unwrap();

    assert!(ir.blocks().len() > 2);
    for (position, block) in ir.blocks().enumerate() {
        assert_eq!(block.id().index(), u32::try_from(position).unwrap());
        assert!(std::ptr::eq(ir.block(block.id()).unwrap(), block));
    }
}

#[test]
fn block_lookup_by_identity_is_total_within_range() {
    let methods = [
        method(
            [(0, Instruction::IConst0), (1, Instruction::IReturn)],
            "()I",
            vec![],
        ),
        branch_and_handler_method(),
        method([(0, Instruction::Goto(0.into()))], "()V", vec![]),
    ];

    for method in &methods {
        let ir = build(method).unwrap();
        let past_the_end = u32::try_from(ir.blocks().len()).unwrap();

        assert!(ir.block(BlockId::new(0)).is_some());
        assert!(ir.block(BlockId::new(past_the_end)).is_none());
        assert!(ir.block(BlockId::new(u32::MAX)).is_none());
    }
}

#[test]
fn rebuild_is_deterministic_and_operations_follow_source_order() {
    let method = branch_and_handler_method();
    let first = build(&method).unwrap();
    let second = build(&method).unwrap();

    assert_eq!(block_shape(&first), block_shape(&second));

    for block in first.blocks() {
        let pcs = block
            .operations()
            .iter()
            .filter_map(|operation| first.source_map().origins_of(operation.id()).next())
            .collect::<Vec<_>>();

        assert_eq!(pcs.len(), block.operations().len());
        assert!(pcs.windows(2).all(|pair| pair[0] < pair[1]));
    }
}

#[test]
fn every_source_program_counter_belongs_to_exactly_one_block() {
    let ir = build(&branch_and_handler_method()).unwrap();
    let source_map = ir.source_map();
    let owner = ir
        .blocks()
        .flat_map(|block| {
            block
                .operations()
                .iter()
                .map(Operation::id)
                .chain(once(block.terminator().id()))
                .map(move |id| (id, block.id()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut blocks_by_pc = BTreeMap::<ProgramCounter, BTreeSet<BlockId>>::new();

    for (id, block) in &owner {
        for pc in source_map.origins_of(*id) {
            blocks_by_pc.entry(pc).or_default().insert(*block);
            assert!(
                source_map
                    .instructions_at(pc)
                    .all(|reported| owner.get(&reported) == Some(block)),
                "{pc} mixes blocks {block} and another block"
            );
        }
    }

    assert!(blocks_by_pc.len() > 2);
    assert!(blocks_by_pc.values().all(|blocks| blocks.len() == 1));
}
