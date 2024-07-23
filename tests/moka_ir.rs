#![cfg(integration_test)]

use mokapot::{
    ir::{
        expression::Expression, DefUseChain, Identifier, LocalValue, MokaIRMethodExt,
        MokaInstruction, Operand,
    },
    jvm::{code::ProgramCounter, Class, ConstantValue, JavaString, Method},
};
use petgraph::dot::Dot;
use proptest::{arbitrary::any, proptest};

fn get_test_class() -> Class {
    let bytes = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/mokapot/java_classes/org/mokapot/test/TestAnalysis.class"
    ));
    Class::from_reader(&bytes[..]).unwrap()
}

fn get_test_method() -> Method {
    let class = get_test_class();
    class
        .methods
        .into_iter()
        .find(|it| it.name == "test")
        .unwrap()
}

#[test]
fn load_test_method() {
    get_test_method();
}

#[test]
fn brew_ir() {
    let class = get_test_class();
    let method = get_test_method();
    let ir = method.brew().unwrap();
    if cfg!(debug_assertions) {
        for (pc, insn) in method.body.unwrap().instructions {
            let ir_insn = ir.instructions.get(&pc).unwrap();
            println!("{}: {:16} => {}", pc, insn.name(), ir_insn)
        }
    }
    let ir_insns = ir.instructions;
    eprintln!("BSM: {:#?}", class.bootstrap_methods);

    eprintln!("methods: {:?}", class.methods);
    // eprintln!("IR instructions: {:?}", ir_insns);
    // #0000: ldc => %0 = String("233")
    assert!(matches!(
        ir_insns.get(&ProgramCounter::from(0x0000)).unwrap(),
        MokaInstruction::Definition {
            value,
            expr: Expression::Const(ConstantValue::String(JavaString::Utf8(str)))
        } if value == &LocalValue::new(0) && str == "233"
    ));
    // #0078: aload => nop
    assert!(matches!(
        ir_insns.get(&ProgramCounter::from(0x007B)).unwrap(),
        MokaInstruction::Nop
    ));
    // #00F7 ireturn => return %arg1
    assert!(matches!(
        ir_insns.get(&ProgramCounter::from(0x00F7)).unwrap(),
        MokaInstruction::Return(Some(Operand::Just(Identifier::Arg(1))))
    ));
}

proptest! {

    #[test]
    fn du_chain_defs(local_idx in any::<u16>()) {
        let method = get_test_method();
        let ir_method = method.brew().unwrap();
        let du_chain = DefUseChain::new(&ir_method);
        let pc = ProgramCounter::from(local_idx);
        if let Some(MokaInstruction::Definition { .. }) = ir_method.instructions.get(&pc) {
            assert_eq!(du_chain.defined_at(&LocalValue::new(local_idx)), Some(pc));
        } else {
            assert!(du_chain.defined_at(&LocalValue::new(local_idx)).is_none());
        }
    }

}

#[test]
fn du_chain_uses() {
    let method = get_test_method();
    let ir_method = method.brew().unwrap();
    let du_chain = DefUseChain::new(&ir_method);
    let test_data = [
        (3, 0x09),
        (24, 0x1F),
        (56, 0x3C),
        (103, 0x68),
        (108, 0x6D),
        (124, 0x7D),
    ];
    for (local_idx, pc) in test_data {
        let pc = ProgramCounter::from(pc);
        assert!(matches!(
            du_chain.used_at(&Identifier::Local(LocalValue::new(local_idx))),
            Some(uses) if uses.contains(&pc)
        ));
    }
}

#[test]
#[cfg(feature = "petgraph")]
fn cfg_to_dot() {
    use itertools::Itertools;
    use mokapot::ir::control_flow::{path_condition, ControlTransfer};

    let method = get_test_method();
    let ir = method.brew().unwrap();
    let condition = ir.control_flow_graph.path_conditions();
    let cfg_with_insn = ir.control_flow_graph.clone().map(
        |pc, _| {
            format!(
                "{pc}: {}\n({})",
                ir.instructions.get(&pc).expect("No instruction"),
                condition.get(&pc).expect("No path condition")
            )
        },
        |_, d| match d {
            ControlTransfer::Unconditional => "".to_owned(),
            ControlTransfer::Conditional(cond) => format!("when {cond}"),
            ControlTransfer::Exception(e) => format!(
                "catch {}",
                e.into_iter().map(|it| it.binary_name).join(" | ")
            ),
            ControlTransfer::SubroutineReturn => "<ret>".to_owned(),
        },
    );
    let dot = Dot::with_config(&cfg_with_insn, &[]);
    println!("{}", dot);
}

#[test]
#[cfg(feature = "petgraph")]
fn dominance() {
    use mokapot::jvm::code::ProgramCounter;

    let method = get_test_method();
    let ir = method.brew().unwrap();
    let _dominance =
        petgraph::algo::dominators::simple_fast(&ir.control_flow_graph, ProgramCounter::ZERO);
}
