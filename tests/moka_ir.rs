use mokapot::{
    ir::MokaIRMethodExt,
    jvm::{class::Class, method::Method},
};
use petgraph::dot::Dot;

fn get_test_class() -> Class {
    let bytes = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/java_classes/org/mokapot/test/TestAnalysis.class"
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
    let ir = method.generate_moka_ir().unwrap();
    for (pc, insn) in method.body.unwrap().instructions {
        let ir_insn = ir.instructions.get(&pc).unwrap();
        println!("{}: {:16} => {}", pc, insn.name(), ir_insn)
    }
}

#[test]
#[cfg(feature = "petgraph")]
fn cfg_to_dot() {
    use mokapot::ir::control_flow::ControlTransfer;

    let method = get_test_method();
    let ir = method.generate_moka_ir().unwrap();
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
            ControlTransfer::Exception(e) => format!("catch {e}"),
            ControlTransfer::SubroutineReturn => "<ret>".to_owned(),
        },
    );
    let dot = Dot::with_config(&cfg_with_insn, &[]);
    println!("{}", dot);
}
