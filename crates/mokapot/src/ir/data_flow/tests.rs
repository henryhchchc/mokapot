use std::collections::BTreeSet;

use super::*;
use crate::{
    ir::TerminatorKind,
    jvm::{
        code::{ExceptionTableEntry, Instruction},
        method::AccessFlags,
    },
    tests::method,
};

#[test]
fn records_parameter_phi_and_terminator_data_flow() {
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
        AccessFlags::PUBLIC | AccessFlags::STATIC,
    );
    let ir = MokaIRMethod::from_method(&method).unwrap();
    let chain = DefUseChain::new(&ir);

    let parameter = ir.parameter_values()[0];
    assert_eq!(
        chain.definition_of(parameter),
        Some(ValueDefinition::Parameter(0))
    );

    let branch = ir.block(ir.entry_block()).unwrap().terminator();
    assert!(matches!(branch.kind(), TerminatorKind::Branch));
    assert_eq!(
        chain.uses_of(parameter).collect::<BTreeSet<_>>(),
        BTreeSet::from([UseSite::Instruction(branch.id())])
    );

    let (join, phi) = ir
        .blocks()
        .find_map(|block| match block.terminator().kind() {
            TerminatorKind::Return(Some(_)) => block.phis().first().map(|phi| (block, phi)),
            _ => None,
        })
        .expect("the join must hold the phi feeding its return");
    assert_eq!(
        chain.definition_of(phi.value),
        Some(ValueDefinition::Instruction(phi.id))
    );
    assert_eq!(
        chain.uses_of(phi.value).collect::<BTreeSet<_>>(),
        BTreeSet::from([UseSite::Instruction(join.terminator().id())])
    );
    for &PhiInput { predecessor, value } in &phi.inputs {
        let site = UseSite::PhiInput {
            phi: phi.id,
            predecessor,
        };
        assert_eq!(
            chain.uses_of(value).collect::<BTreeSet<_>>(),
            BTreeSet::from([site])
        );
        assert_eq!(site.instruction(), phi.id);
    }
}

#[test]
fn records_the_this_definition_and_its_terminator_use() {
    let method = method(
        [(0, Instruction::ALoad0), (1, Instruction::AReturn)],
        "()Ljava/lang/Object;",
        vec![],
        AccessFlags::PUBLIC,
    );
    let ir = MokaIRMethod::from_method(&method).unwrap();
    let chain = DefUseChain::new(&ir);

    let this = ir
        .this_value()
        .expect("an instance method has a this value");
    let returned = ir
        .blocks()
        .find(|block| {
            matches!(block.terminator().kind(), TerminatorKind::Return(Some(value)) if *value == this)
        })
        .expect("the method must return this")
        .terminator();

    assert_eq!(chain.definition_of(this), Some(ValueDefinition::This));
    assert_eq!(
        chain.uses_of(this).collect::<BTreeSet<_>>(),
        BTreeSet::from([UseSite::Instruction(returned.id())])
    );
}

#[test]
fn records_an_unused_caught_exception_definition() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (
                1,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (2, Instruction::Pop),
            (3, Instruction::Return),
            (10, Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: None,
        }],
        AccessFlags::PUBLIC | AccessFlags::STATIC,
    );
    let ir = MokaIRMethod::from_method(&method).unwrap();
    let chain = DefUseChain::new(&ir);

    let (handler, caught) = ir
        .blocks()
        .find_map(|block| {
            ir.caught_exception(block.id())
                .map(|value| (block.id(), value))
        })
        .expect("the handler entry must define a caught exception");

    assert_eq!(
        chain.definition_of(caught),
        Some(ValueDefinition::CaughtException(handler))
    );
    assert_eq!(chain.uses_of(caught).count(), 0);
}

#[test]
fn returns_none_for_a_value_outside_the_method_dense_identity_range() {
    let method = method(
        [(0, Instruction::Return)],
        "()V",
        vec![],
        AccessFlags::PUBLIC | AccessFlags::STATIC,
    );
    let ir = MokaIRMethod::from_method(&method).unwrap();
    let chain = DefUseChain::new(&ir);

    assert_eq!(chain.definition_of(ValueId::new(u32::MAX)), None);
}
