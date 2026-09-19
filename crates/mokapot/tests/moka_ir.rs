use mokapot::{
    ir::{
        InstructionLocation, InstructionRef, MokaIRMethod, OperationKind, TerminatorKind,
        expression::Expression,
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

fn operation(
    method: &MokaIRMethod,
    location: InstructionLocation,
) -> Option<&mokapot::ir::Operation> {
    match method.instruction(location) {
        Some(InstructionRef::Operation(operation)) => Some(operation),
        Some(InstructionRef::Phi(_) | InstructionRef::Terminator(_)) | None => None,
    }
}

fn terminator(
    method: &MokaIRMethod,
    location: InstructionLocation,
) -> Option<&mokapot::ir::Terminator> {
    match method.instruction(location) {
        Some(InstructionRef::Terminator(terminator)) => Some(terminator),
        Some(InstructionRef::Phi(_) | InstructionRef::Operation(_)) | None => None,
    }
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
        .find_map(|id| operation(&ir, id))
        .unwrap();
    assert!(matches!(
        first.kind(),
        OperationKind::Definition {
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

    for (block_id, _) in ir.blocks() {
        assert!(ir.block(block_id).is_some());
    }
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn ssa_identities_and_phi_predecessors_are_well_formed() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    let mut instruction_locations = HashSet::new();
    let mut definitions = HashSet::new();
    let mut uses = HashSet::new();

    if let Some(value) = ir.this_value() {
        definitions.insert(value);
    }
    definitions.extend(ir.parameter_values());
    for (block_id, block) in ir.blocks() {
        if let Some(value) = ir.caught_exception(block_id) {
            definitions.insert(value);
        }
        let predecessors = ir
            .blocks()
            .filter(|(_, candidate)| {
                candidate
                    .terminator
                    .successors()
                    .iter()
                    .any(|successor| successor.target() == block_id)
            })
            .map(|(candidate_id, _)| candidate_id)
            .collect::<BTreeSet<_>>();
        for (index, phi) in block.phis.iter().enumerate() {
            assert!(instruction_locations.insert(InstructionLocation::Phi {
                block: block_id,
                index,
            }));
            assert!(definitions.insert(phi.value));
            assert_eq!(
                phi.inputs
                    .iter()
                    .map(|it| it.predecessor)
                    .collect::<BTreeSet<_>>(),
                predecessors
            );
            uses.extend(phi.inputs.iter().map(|it| it.value));
        }
        for (index, instruction) in block.operations.iter().enumerate() {
            assert!(
                instruction_locations.insert(InstructionLocation::Operation {
                    block: block_id,
                    index,
                })
            );
            if let Some(value) = instruction.def() {
                assert!(definitions.insert(value));
            }
            uses.extend(instruction.uses());
        }
        assert!(instruction_locations.insert(InstructionLocation::Terminator { block: block_id }));
        uses.extend(block.terminator.uses());
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
    let mut phis = ir.blocks().flat_map(|(block, bb)| {
        bb.phis
            .iter()
            .enumerate()
            .map(move |(index, _)| InstructionLocation::Phi { block, index })
    });
    assert!(phis.all(|location| {
        ir.source_map().origin_of(location).is_none() && !covered_nodes.contains(&location)
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
            .filter(|(block_id, _)| *block_id != ir.entry_block())
            .all(|(block_id, _)| dominance.immediate_dominator(block_id).is_some())
    );
}
