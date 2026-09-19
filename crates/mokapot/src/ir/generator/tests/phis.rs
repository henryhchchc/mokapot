use super::*;

#[test]
fn diamond_merge_uses_a_block_parameter_and_edge_arguments() {
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
    let [parameter] = join.parameters.as_slice() else {
        panic!("the join must contain one parameter")
    };

    let incoming = ir
        .blocks()
        .flat_map(|(_, bb)| bb.terminator.successors())
        .filter(|it| it.block_target() == Some(join_id))
        .collect::<Vec<_>>();
    assert_eq!(incoming.len(), 2);
    assert!(incoming.iter().all(|edge| edge.arguments().len() == 1));
    assert!(matches!(
        join.terminator.kind(),
        TerminatorKind::Return(Some(value)) if *value == parameter.value
    ));
    let join_loc = InstructionLocation::BlockParameter {
        block: join_id,
        index: 0,
    };
    assert_eq!(
        ir.definition_of(parameter.value),
        Some(ValueDefinition::Instruction(join_loc))
    );
    let join_loc = InstructionLocation::BlockParameter {
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
fn entry_backedge_uses_method_entry_arguments_and_a_loop_parameter() {
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
    let header_id = ir.entry_block();
    let header = ir.block(header_id).unwrap();
    let [parameter] = header.parameters.as_slice() else {
        panic!("the loop header must contain one parameter")
    };

    assert_eq!(ir.entry().target(), header_id);
    assert_eq!(ir.entry().arguments(), ir.parameter_values());
    let backedge_value = ir
        .blocks()
        .flat_map(|(_, bb)| bb.terminator.successors())
        .filter(|it| it.block_target() == Some(header_id))
        .flat_map(Successor::arguments)
        .copied()
        .find(|&value| value != ir.entry().arguments()[0])
        .unwrap();
    let Some(ValueDefinition::Instruction(backedge_definition)) = ir.definition_of(backedge_value)
    else {
        panic!("the loop-carried input must be computed in the loop")
    };
    let InstructionLocation::Operation { block, index } = backedge_definition else {
        panic!("the loop-carried input must be computed by an operation")
    };
    let definition = &ir.block(block).unwrap().operations[index];
    assert!(definition.uses().contains(&parameter.value));
}

#[test]
fn entry_self_loop_needs_no_synthetic_block_or_redundant_parameters() {
    let method = method([(0, Instruction::Goto(0.into()))], "()V", vec![]);
    let ir = build(&method).unwrap();
    let header_id = ir.entry_block();
    let header = ir.block(header_id).unwrap();

    assert_eq!(ir.blocks().len(), 1);
    assert!(header.parameters.is_empty());
    assert!(ir.entry().arguments().is_empty());
    assert_eq!(header.terminator.successors().len(), 1);
    assert!(matches!(
        header.terminator.successors()[0].transfer(),
        ControlTransfer::Unconditional
    ));
    assert_eq!(
        header.terminator.successors()[0].block_target(),
        Some(header_id)
    );
    let loc = InstructionLocation::Terminator { block: header_id };
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

    assert_eq!(header.parameters.len(), 1);
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
    let parameters = ir
        .blocks()
        .flat_map(|(_, block)| &block.parameters)
        .collect::<Vec<_>>();

    assert_eq!(parameters.len(), 3);
    let argument_values = ir
        .blocks()
        .flat_map(|(_, bb)| bb.terminator.successors())
        .flat_map(Successor::arguments)
        .copied()
        .collect::<HashSet<_>>();
    let cyclic_results = parameters
        .iter()
        .filter(|it| argument_values.contains(&it.value))
        .map(|it| it.value)
        .collect::<HashSet<_>>();
    assert_eq!(cyclic_results.len(), 2);
    assert!(
        parameters
            .iter()
            .filter(|it| cyclic_results.contains(&it.value))
            .all(|it| argument_values.contains(&it.value))
    );
}
