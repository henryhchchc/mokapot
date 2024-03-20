use mokapot::{
    ir::MokaIRMethodExt,
    jvm::{class::Class, method::Method},
};
use petgraph::dot::Dot;

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
fn analyze() {
    let method = get_test_method();
    let ir = method.brew().unwrap();
    for (pc, insn) in method.body.unwrap().instructions {
        let ir_insn = ir.instructions.get(&pc).unwrap();
        println!("{}: {:16} => {}", pc, insn.name(), ir_insn)
    }
}

#[test]
#[cfg(feature = "petgraph")]
fn cfg_to_dot() {
    use itertools::Itertools;
    use mokapot::ir::control_flow::ControlTransfer;

    let method = get_test_method();
    let ir = method.brew().unwrap();
    let cfg_with_insn = ir.control_flow_graph.clone().map(
        |pc, _| {
            format!(
                "{pc}: {}",
                ir.instructions.get(&pc).expect("No instruction")
            )
        },
        |_, d| match d {
            ControlTransfer::Unconditional => "".to_owned(),
            ControlTransfer::Conditional => "<conditional>".to_owned(),
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
