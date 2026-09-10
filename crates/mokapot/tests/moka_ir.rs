use mokapot::{
    ir::{
        DefUseChain, InstructionId, InstructionKind, MokaIRMethod, TerminatorKind, UseSite,
        ValueDefinition, expression::Expression,
    },
    jvm::{Class, ConstantValue, JavaString, Method, code::ProgramCounter},
};
use std::collections::{BTreeSet, HashSet};

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

fn instruction(method: &MokaIRMethod, id: InstructionId) -> Option<&mokapot::ir::Instruction> {
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
fn builds_ir_blocks_and_provenance() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();

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

    assert_eq!(
        ir.source_map()
            .instructions_at(ProgramCounter::from(0x007B))
            .count(),
        0
    );

    let returned = ir
        .source_map()
        .instructions_at(ProgramCounter::from(0x00F7))
        .find_map(|id| terminator(&ir, id))
        .unwrap();
    assert!(matches!(
        returned.kind(),
        TerminatorKind::Return(Some(value)) if value == &ir.parameter_values()[1]
    ));

    for block in ir.blocks() {
        assert!(ir.block(block.id()).is_some());
    }
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn du_chain_definitions_use_instruction_identities() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    let chain = DefUseChain::new(&ir);
    for instruction in ir.blocks().flat_map(|block| block.instructions()) {
        if let Some(value) = instruction.def() {
            assert_eq!(
                chain.definition_of(value),
                Some(ValueDefinition::Instruction(instruction.id()))
            );
        }
    }
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn ssa_identities_and_phi_predecessors_are_well_formed() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    let mut instruction_ids = HashSet::new();
    let mut definitions = HashSet::new();
    let mut uses = HashSet::new();

    if let Some(value) = ir.this_value() {
        definitions.insert(value);
    }
    definitions.extend(ir.parameter_values());
    for block in ir.blocks() {
        if let Some(value) = ir.caught_exception(block.id()) {
            definitions.insert(value);
        }
        let predecessors = ir
            .blocks()
            .filter(|candidate| {
                candidate
                    .terminator()
                    .successors()
                    .iter()
                    .any(|successor| successor.target() == block.id())
            })
            .map(mokapot::ir::BasicBlock::id)
            .collect::<BTreeSet<_>>();
        for phi in block.phis() {
            assert!(instruction_ids.insert(phi.id()));
            assert!(definitions.insert(phi.value()));
            assert_eq!(
                phi.inputs()
                    .iter()
                    .map(mokapot::ir::PhiInput::predecessor)
                    .collect::<BTreeSet<_>>(),
                predecessors
            );
            uses.extend(phi.inputs().iter().map(mokapot::ir::PhiInput::value));
        }
        for instruction in block.instructions() {
            assert!(instruction_ids.insert(instruction.id()));
            if let Some(value) = instruction.def() {
                assert!(definitions.insert(value));
            }
            uses.extend(instruction.uses());
        }
        assert!(instruction_ids.insert(block.terminator().id()));
        uses.extend(block.terminator().uses());
    }

    assert!(uses.iter().all(|value| definitions.contains(value)));
    assert!(
        definitions
            .iter()
            .all(|value| ir.definition_of(*value).is_some())
    );
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn du_chain_uses_include_source_related_nodes() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
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
            .find_map(|id| instruction(&ir, id).and_then(mokapot::ir::Instruction::def))
            .unwrap();
        let uses = chain.uses_of(value).collect::<BTreeSet<_>>();
        assert!(
            ir.source_map()
                .instructions_at(ProgramCounter::from(use_pc))
                .any(|id| uses.contains(&UseSite::Instruction(id)))
        );
    }
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn coverage_transfer_uses_only_sparse_source_provenance() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    let covered_pcs = [ProgramCounter::from(0x0000), ProgramCounter::from(0x0003)];
    let covered_nodes = covered_pcs
        .into_iter()
        .flat_map(|pc| ir.source_map().instructions_at(pc))
        .collect::<BTreeSet<_>>();

    assert!(!covered_nodes.is_empty());
    assert!(covered_pcs.into_iter().all(|pc| {
        ir.source_map()
            .instructions_at(pc)
            .all(|instruction| covered_nodes.contains(&instruction))
    }));
    assert_eq!(
        ir.source_map()
            .instructions_at(ProgramCounter::from(0x007B))
            .count(),
        0
    );
    assert!(ir.blocks().flat_map(|block| block.phis()).all(|phi| {
        ir.source_map().origins_of(phi.id()).next().is_none() && !covered_nodes.contains(&phi.id())
    }));
}

#[test]
#[cfg(feature = "petgraph")]
#[cfg_attr(not(integration_test), ignore)]
fn cfg_to_dot() {
    use petgraph::dot::Dot;

    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    let cfg = ir.control_flow_graph();
    let dot = format!("{:?}", Dot::new(&cfg));
    assert!(dot.contains("digraph"));
    assert!(!cfg.path_conditions().is_empty());
}

#[test]
#[cfg(feature = "petgraph")]
#[cfg_attr(not(integration_test), ignore)]
fn dominance() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    let cfg = ir.control_flow_graph();
    let dominance = petgraph::algo::dominators::simple_fast(&cfg, ir.entry_block());
    assert_eq!(dominance.immediate_dominator(ir.entry_block()), None);
    assert!(
        ir.blocks()
            .filter(|block| block.id() != ir.entry_block())
            .all(|block| { dominance.immediate_dominator(block.id()).is_some() })
    );
}
