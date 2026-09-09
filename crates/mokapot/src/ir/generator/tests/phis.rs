#[allow(
    clippy::wildcard_imports,
    reason = "generator tests share the fixture helpers from their parent module"
)]
use super::*;

#[test]
fn diamond_merge_uses_a_predecessor_indexed_phi() {
    let method = method(
        [
            (0.into(), Instruction::ILoad0),
            (1.into(), Instruction::IfEq(5.into())),
            (2.into(), Instruction::IConst1),
            (3.into(), Instruction::Goto(6.into())),
            (5.into(), Instruction::IConst2),
            (6.into(), Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );
    let ir = method.brew().unwrap();
    let join = ir
        .blocks()
        .find(|block| matches!(block.terminator().kind(), TerminatorKind::Return(Some(_))))
        .unwrap();
    let [phi] = join.phis() else {
        panic!("the join must contain one phi")
    };

    assert_eq!(phi.inputs().len(), 2);
    assert_ne!(phi.inputs()[0].predecessor(), phi.inputs()[1].predecessor());
    assert!(matches!(
        join.terminator().kind(),
        TerminatorKind::Return(Some(value)) if *value == phi.value()
    ));
    assert_eq!(
        ir.value_definition(phi.value()),
        Some(ValueDefinition::Instruction(phi.id()))
    );
    assert_eq!(ir.source_map().origins_of(phi.id()).count(), 0);
}

#[test]
fn value_missing_on_one_predecessor_cannot_be_used_at_the_join() {
    let method = method(
        [
            (0.into(), Instruction::ILoad0),
            (1.into(), Instruction::IfEq(5.into())),
            (2.into(), Instruction::IConst1),
            (3.into(), Instruction::IStore1),
            (4.into(), Instruction::Goto(6.into())),
            (5.into(), Instruction::Nop),
            (6.into(), Instruction::ILoad1),
            (7.into(), Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );

    assert!(matches!(
        method.brew(),
        Err(MokaIRBrewingError::ExecutionError(_))
    ));
}

#[test]
fn entry_backedge_gets_a_synthetic_preheader_and_loop_phi() {
    let method = method(
        [
            (0.into(), Instruction::ILoad0),
            (1.into(), Instruction::IfEq(8.into())),
            (2.into(), Instruction::ILoad0),
            (3.into(), Instruction::IConst1),
            (4.into(), Instruction::ISub),
            (5.into(), Instruction::IStore0),
            (6.into(), Instruction::Goto(0.into())),
            (8.into(), Instruction::ILoad0),
            (9.into(), Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );
    let ir = method.brew().unwrap();
    let preheader = ir.block(ir.entry_block()).unwrap();
    let header_id = preheader.terminator().successors()[0].target();
    let header = ir.block(header_id).unwrap();
    let [phi] = header.phis() else {
        panic!("the loop header must contain one phi")
    };

    assert_eq!(preheader.instructions().len(), 0);
    assert_eq!(
        ir.source_map()
            .origins_of(preheader.terminator().id())
            .count(),
        0
    );
    assert_eq!(phi.inputs().len(), 2);
    assert!(
        phi.inputs()
            .iter()
            .any(|input| input.predecessor() == ir.entry_block())
    );
    let backedge_value = phi
        .inputs()
        .iter()
        .find(|input| input.predecessor() != ir.entry_block())
        .unwrap()
        .value();
    let Some(ValueDefinition::Instruction(backedge_definition)) =
        ir.value_definition(backedge_value)
    else {
        panic!("the loop-carried input must be computed in the loop")
    };
    let definition = ir
        .blocks()
        .flat_map(BasicBlock::instructions)
        .find(|instruction| instruction.id() == backedge_definition)
        .unwrap();
    assert!(definition.uses().contains(&phi.value()));
}

#[test]
fn entry_self_loop_gets_a_preheader_without_redundant_phis() {
    let method = method([(0.into(), Instruction::Goto(0.into()))], "()V", vec![]);
    let ir = method.brew().unwrap();
    let blocks = ir.blocks().collect::<Vec<_>>();

    assert_eq!(blocks.len(), 2);
    assert!(blocks.iter().all(|block| block.phis().is_empty()));
    assert!(blocks[0].instructions().is_empty());
    assert_eq!(blocks[0].terminator().successors().len(), 1);
    assert!(matches!(
        blocks[0].terminator().successors()[0].transfer(),
        ControlTransfer::Unconditional
    ));
    assert_eq!(
        ir.source_map()
            .origins_of(blocks[0].terminator().id())
            .count(),
        0
    );
}

#[test]
fn mutually_recursive_trivial_phis_collapse_in_a_loop() {
    let method = method(
        [
            (0.into(), Instruction::ILoad1),
            (1.into(), Instruction::IStore2),
            (2.into(), Instruction::ILoad1),
            (3.into(), Instruction::IStore3),
            (4.into(), Instruction::ILoad0),
            (5.into(), Instruction::IfEq(20.into())),
            (8.into(), Instruction::ILoad2),
            (9.into(), Instruction::ILoad3),
            (10.into(), Instruction::IStore2),
            (11.into(), Instruction::IStore3),
            (12.into(), Instruction::IInc(0, -1)),
            (15.into(), Instruction::Goto(4.into())),
            (20.into(), Instruction::ILoad2),
            (21.into(), Instruction::IReturn),
        ],
        "(II)I",
        vec![],
    );
    let ir = method.brew().unwrap();
    let header = ir
        .blocks()
        .find(|block| matches!(block.terminator().kind(), TerminatorKind::Branch))
        .unwrap();

    assert_eq!(header.phis().len(), 1);
    let counter = &header.phis()[0];
    assert_eq!(counter.inputs().len(), 2);
    let returned = ir
        .blocks()
        .find_map(|block| match block.terminator().kind() {
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
            (0.into(), Instruction::ILoad0),
            (1.into(), Instruction::IfEq(10.into())),
            (4.into(), Instruction::IConst1),
            (5.into(), Instruction::IStore1),
            (6.into(), Instruction::Goto(20.into())),
            (10.into(), Instruction::IConst2),
            (11.into(), Instruction::IStore1),
            (12.into(), Instruction::Goto(30.into())),
            (20.into(), Instruction::ILoad0),
            (21.into(), Instruction::IfEq(30.into())),
            (24.into(), Instruction::Goto(40.into())),
            (30.into(), Instruction::ILoad0),
            (31.into(), Instruction::IfEq(20.into())),
            (34.into(), Instruction::Goto(40.into())),
            (40.into(), Instruction::ILoad1),
            (41.into(), Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );
    let ir = method.brew().unwrap();
    let phis = ir.blocks().flat_map(BasicBlock::phis).collect::<Vec<_>>();

    assert_eq!(phis.len(), 3);
    let cyclic_results = phis
        .iter()
        .filter(|candidate| {
            phis.iter().any(|phi| {
                phi.inputs()
                    .iter()
                    .any(|input| input.value() == candidate.value())
            })
        })
        .map(|phi| phi.value())
        .collect::<HashSet<_>>();
    assert_eq!(cyclic_results.len(), 2);
    assert!(
        phis.iter()
            .filter(|phi| cyclic_results.contains(&phi.value()))
            .all(|phi| {
                phi.inputs().len() == 2
                    && phi
                        .inputs()
                        .iter()
                        .any(|input| cyclic_results.contains(&input.value()))
            })
    );
}
