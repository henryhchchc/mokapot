use super::*;

#[test]
fn diamond_merge_uses_a_predecessor_indexed_phi() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::IfEq(5.into())),
            (2, Instruction::IConst1),
            (3, Instruction::Goto(6.into())),
            (5, Instruction::IConst2),
            (6, Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );
    let ir = build(&method).unwrap();
    let (join_id, join) = ir
        .blocks()
        .find(|(_, block)| matches!(block.terminator.kind(), TerminatorKind::Return(Some(_))))
        .unwrap();
    let [phi] = join.phis.as_slice() else {
        panic!("the join must contain one phi")
    };

    assert_eq!(phi.inputs.len(), 2);
    assert_ne!(phi.inputs[0].predecessor, phi.inputs[1].predecessor);
    assert!(matches!(
        join.terminator.kind(),
        TerminatorKind::Return(Some(value)) if *value == phi.value
    ));
    let join_loc = InstructionLocation::Phi {
        block: join_id,
        index: 0,
    };
    assert_eq!(
        ir.definition_of(phi.value),
        Some(ValueDefinition::Instruction(join_loc))
    );
    let join_loc = InstructionLocation::Phi {
        block: join_id,
        index: 0,
    };
    assert_eq!(ir.source_map().origin_of(join_loc), None);
}

#[test]
fn value_missing_on_one_predecessor_cannot_be_used_at_the_join() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::IfEq(5.into())),
            (2, Instruction::IConst1),
            (3, Instruction::IStore1),
            (4, Instruction::Goto(6.into())),
            (5, Instruction::Nop),
            (6, Instruction::ILoad1),
            (7, Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::InvalidFrame {
            pc: Some(pc),
            ..
        }) if pc == 6.into()
    ));
}

#[test]
fn entry_backedge_gets_a_synthetic_preheader_and_loop_phi() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::IfEq(8.into())),
            (2, Instruction::ILoad0),
            (3, Instruction::IConst1),
            (4, Instruction::ISub),
            (5, Instruction::IStore0),
            (6, Instruction::Goto(0.into())),
            (8, Instruction::ILoad0),
            (9, Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );
    let ir = build(&method).unwrap();
    let preheader = ir.block(ir.entry_block()).unwrap();
    let header_id = preheader.terminator.successors()[0].target();
    let header = ir.block(header_id).unwrap();
    let [phi] = header.phis.as_slice() else {
        panic!("the loop header must contain one phi")
    };

    assert_eq!(preheader.operations.len(), 0);
    let loc = InstructionLocation::Terminator {
        block: ir.entry_block(),
    };
    assert_eq!(ir.source_map().origin_of(loc), None);
    assert_eq!(phi.inputs.len(), 2);
    assert!(
        phi.inputs
            .iter()
            .any(|input| input.predecessor == ir.entry_block())
    );
    let backedge_value = phi
        .inputs
        .iter()
        .find(|input| input.predecessor != ir.entry_block())
        .unwrap()
        .value;
    let Some(ValueDefinition::Instruction(backedge_definition)) = ir.definition_of(backedge_value)
    else {
        panic!("the loop-carried input must be computed in the loop")
    };
    let InstructionLocation::Operation { block, index } = backedge_definition else {
        panic!("the loop-carried input must be computed by an operation")
    };
    let definition = &ir.block(block).unwrap().operations[index];
    assert!(definition.uses().contains(&phi.value));
}

#[test]
fn entry_self_loop_gets_block_zero_preheader_without_redundant_phis() {
    let method = method([(0, Instruction::Goto(0.into()))], "()V", vec![]);
    let ir = build(&method).unwrap();
    let blocks = ir.blocks().collect::<Vec<_>>();

    assert_eq!(blocks.len(), 2);
    assert!(blocks.iter().all(|(_, block)| block.phis.is_empty()));
    assert!(blocks[0].1.operations.is_empty());
    assert_eq!(blocks[0].1.terminator.successors().len(), 1);
    assert!(matches!(
        blocks[0].1.terminator.successors()[0].transfer(),
        ControlTransfer::Unconditional
    ));
    let loc = InstructionLocation::Terminator { block: blocks[0].0 };
    assert_eq!(ir.source_map().origin_of(loc), None);
    let [arm] = blocks[0].1.terminator.successors() else {
        panic!("the preheader must have exactly one successor")
    };
    let header = ir.block(arm.target()).unwrap();
    assert_eq!(header, blocks[1].1);
    assert_eq!(header.terminator.successors()[0].target(), blocks[1].0);
    let loc = InstructionLocation::Terminator { block: blocks[1].0 };
    assert_eq!(
        ir.source_map().origin_of(loc),
        Some(ProgramCounter::from(0))
    );
}

#[test]
fn mutually_recursive_trivial_phis_collapse_in_a_loop() {
    let method = method(
        [
            (0, Instruction::ILoad1),
            (1, Instruction::IStore2),
            (2, Instruction::ILoad1),
            (3, Instruction::IStore3),
            (4, Instruction::ILoad0),
            (5, Instruction::IfEq(20.into())),
            (8, Instruction::ILoad2),
            (9, Instruction::ILoad3),
            (10, Instruction::IStore2),
            (11, Instruction::IStore3),
            (12, Instruction::IInc(0, -1)),
            (15, Instruction::Goto(4.into())),
            (20, Instruction::ILoad2),
            (21, Instruction::IReturn),
        ],
        "(II)I",
        vec![],
    );
    let ir = build(&method).unwrap();
    let header = ir
        .blocks()
        .map(|(_, block)| block)
        .find(|block| matches!(block.terminator.kind(), TerminatorKind::Branch))
        .unwrap();

    assert_eq!(header.phis.len(), 1);
    let counter = &header.phis[0];
    assert_eq!(counter.inputs.len(), 2);
    let returned = ir
        .blocks()
        .map(|(_, block)| block)
        .find_map(|block| match block.terminator.kind() {
            TerminatorKind::Return(Some(value)) => Some(value),
            _ => None,
        })
        .unwrap();
    assert_eq!(*returned, ir.parameter_values()[1]);
}

#[test]
fn irreducible_loop_retains_a_finite_cyclic_phi_pair() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::IfEq(10.into())),
            (4, Instruction::IConst1),
            (5, Instruction::IStore1),
            (6, Instruction::Goto(20.into())),
            (10, Instruction::IConst2),
            (11, Instruction::IStore1),
            (12, Instruction::Goto(30.into())),
            (20, Instruction::ILoad0),
            (21, Instruction::IfEq(30.into())),
            (24, Instruction::Goto(40.into())),
            (30, Instruction::ILoad0),
            (31, Instruction::IfEq(20.into())),
            (34, Instruction::Goto(40.into())),
            (40, Instruction::ILoad1),
            (41, Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );
    let ir = build(&method).unwrap();
    let phis = ir
        .blocks()
        .flat_map(|(_, block)| &block.phis)
        .collect::<Vec<_>>();

    assert_eq!(phis.len(), 3);
    let cyclic_results = phis
        .iter()
        .filter(|candidate| {
            phis.iter().any(|phi| {
                phi.inputs
                    .iter()
                    .any(|input| input.value == candidate.value)
            })
        })
        .map(|phi| phi.value)
        .collect::<HashSet<_>>();
    assert_eq!(cyclic_results.len(), 2);
    assert!(
        phis.iter()
            .filter(|phi| cyclic_results.contains(&phi.value))
            .all(|phi| {
                phi.inputs.len() == 2
                    && phi
                        .inputs
                        .iter()
                        .any(|input| cyclic_results.contains(&input.value))
            })
    );
}
