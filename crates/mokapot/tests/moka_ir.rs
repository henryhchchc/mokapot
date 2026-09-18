use mokapot::{
    ir::{
        DefUseChain, Identifier, InstructionId, InstructionKind, MokaIRMethod, MokaIRMethodExt,
        TerminatorKind, expression::Expression,
    },
    jvm::{Class, ConstantValue, JavaString, Method, code::ProgramCounter},
};

fn get_test_class() -> Class {
    let mut bytes = if cfg!(integration_test) {
        include_bytes!(concat!(
            env!("OUT_DIR"),
            "/mokapot/java_classes/org/mokapot/test/TestAnalysis.class"
        ))
        .as_slice()
    } else {
        &[]
    };
    Class::from_reader(&mut bytes).unwrap()
}

fn get_test_method() -> Method {
    get_test_class()
        .methods
        .into_iter()
        .find(|method| method.name == "test")
        .unwrap()
}

fn instruction(method: &MokaIRMethod, id: InstructionId) -> Option<&mokapot::ir::MokaInstruction> {
    method
        .blocks()
        .flat_map(|block| block.instructions())
        .find(|instruction| instruction.id() == id)
}

fn terminator(method: &MokaIRMethod, id: InstructionId) -> Option<&mokapot::ir::Terminator> {
    method
        .blocks()
        .map(mokapot::ir::BasicBlock::terminator)
        .find(|terminator| terminator.id() == id)
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn load_test_method() {
    get_test_method();
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn brew_ir_blocks_and_provenance() {
    let ir = get_test_method().brew().unwrap();

    let first = ir
        .source_map()
        .instructions_at(ProgramCounter::from(0x0000))
        .find_map(|id| instruction(&ir, id))
        .unwrap();
    assert!(matches!(
        first.kind(),
        InstructionKind::Definition {
            expr: Expression::Const(ConstantValue::String(JavaString::Utf8(value))),
            ..
        } if value == "233"
    ));

    let nop = ir
        .source_map()
        .instructions_at(ProgramCounter::from(0x007B))
        .find_map(|id| instruction(&ir, id))
        .unwrap();
    assert_eq!(nop.kind(), &InstructionKind::Nop);

    let returned = ir
        .source_map()
        .instructions_at(ProgramCounter::from(0x00F7))
        .find_map(|id| terminator(&ir, id))
        .unwrap();
    assert!(matches!(
        returned.kind(),
        TerminatorKind::Return(Some(value)) if value == &Identifier::Arg(1).into()
    ));

    for block in ir.blocks() {
        assert!(ir.block(block.id()).is_some());
    }
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn du_chain_definitions_use_instruction_identities() {
    let ir = get_test_method().brew().unwrap();
    let chain = DefUseChain::new(&ir);
    for instruction in ir.blocks().flat_map(|block| block.instructions()) {
        if let Some(value) = instruction.def() {
            assert_eq!(chain.defined_at(value), Some(instruction.id()));
        }
    }
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn du_chain_uses_include_source_related_nodes() {
    let ir = get_test_method().brew().unwrap();
    let chain = DefUseChain::new(&ir);
    let test_data = [
        (3, 0x09),
        (24, 0x1F),
        (56, 0x3C),
        (103, 0x68),
        (108, 0x6D),
        (124, 0x7D),
    ];
    for (definition_pc, use_pc) in test_data {
        let value = ir
            .source_map()
            .instructions_at(ProgramCounter::from(definition_pc))
            .find_map(|id| instruction(&ir, id).and_then(mokapot::ir::MokaInstruction::def))
            .unwrap();
        let uses = chain.used_at(Identifier::Local(value));
        assert!(
            ir.source_map()
                .instructions_at(ProgramCounter::from(use_pc))
                .any(|id| uses.contains(&id))
        );
    }
}

#[test]
#[cfg(feature = "petgraph")]
#[cfg_attr(not(integration_test), ignore)]
fn cfg_to_dot() {
    use petgraph::dot::Dot;

    let ir = get_test_method().brew().unwrap();
    let cfg = ir.control_flow_graph();
    let dot = format!("{:?}", Dot::new(&cfg));
    assert!(dot.contains("digraph"));
    assert!(!cfg.path_conditions().is_empty());
}

#[test]
#[cfg(feature = "petgraph")]
#[cfg_attr(not(integration_test), ignore)]
fn dominance() {
    let ir = get_test_method().brew().unwrap();
    let cfg = ir.control_flow_graph();
    let dominance = petgraph::algo::dominators::simple_fast(&cfg, ir.entry_block());
    assert_eq!(dominance.immediate_dominator(ir.entry_block()), None);
}
