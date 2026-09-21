#![allow(missing_docs, clippy::ignore_without_reason)]

use std::collections::{HashSet, VecDeque};

use mokapot::{
    ir::{
        InstructionLocation, InstructionRef, MokaIRMethod, Operation, Terminator,
        expression::Expression,
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

fn terminator(
    method: &MokaIRMethod,
    location: InstructionLocation,
) -> Option<&mokapot::ir::Terminator> {
    match method.instruction(location) {
        Some(InstructionRef::Terminator(terminator)) => Some(terminator),
        Some(InstructionRef::BlockParameter(_) | InstructionRef::Operation(_)) | None => None,
    }
}

fn reachable_blocks(ir: &MokaIRMethod) -> Vec<(mokapot::ir::BlockId, &mokapot::ir::BasicBlock)> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut pending = VecDeque::from([ir.entry_block()]);
    while let Some(id) = pending.pop_front() {
        if !visited.insert(id) {
            continue;
        }
        let block = ir.block(id).expect("a successor must belong to its method");
        pending.extend(
            block
                .terminator
                .successors()
                .filter_map(mokapot::ir::Successor::block_target),
        );
        result.push((id, block));
    }
    result
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

    // The `ldc` at `0x0000` may fail, so lifting folds it into the block's
    // `Try` terminator instead of an ordinary operation.
    let first = ir
        .source_map()
        .instructions_at(ProgramCounter::from(0x0000))
        .find_map(|id| terminator(&ir, id))
        .and_then(Terminator::operation)
        .unwrap();
    assert!(matches!(
        first,
        Operation::Definition {
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
    // Exiting a method may unwind, so the return is a fallible terminator.
    assert!(matches!(
        returned,
        Terminator::TryReturn { value: Some(value), .. } if value == &ir.parameter_values()[1]
    ));

    for (block_id, _) in reachable_blocks(&ir) {
        assert!(ir.block(block_id).is_some());
    }
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn ssa_definitions_and_block_arguments_are_well_formed() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    let mut instruction_locations = HashSet::new();
    let mut definitions = HashSet::new();
    let mut uses = HashSet::new();

    if let Some(value) = ir.this_value() {
        definitions.insert(value);
    }
    definitions.extend(ir.parameter_values());
    assert_eq!(
        ir.entry().arguments().len(),
        ir.block(ir.entry_block()).unwrap().parameters.len()
    );
    uses.extend(ir.entry().arguments());
    for (block_id, block) in reachable_blocks(&ir) {
        if let mokapot::ir::BlockKind::LandingPad { exception: value } = block.kind {
            definitions.insert(value);
        }
        for (index, parameter) in block.parameters.iter().enumerate() {
            assert!(
                instruction_locations.insert(InstructionLocation::BlockParameter {
                    block: block_id,
                    index,
                })
            );
            assert!(definitions.insert(parameter.value));
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
        if let Some(value) = block.terminator.def() {
            assert!(definitions.insert(value));
        }
        for successor in block.terminator.successors() {
            assert_eq!(
                successor.arguments().len(),
                successor
                    .block_target()
                    .and_then(|target| ir.block(target))
                    .map_or(0, |target| target.parameters.len())
            );
        }
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
        .collect::<HashSet<_>>();

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
    let blocks = reachable_blocks(&ir);
    let mut params = blocks.iter().flat_map(|(block, bb)| {
        bb.parameters.iter().enumerate().map(move |(index, _)| {
            InstructionLocation::BlockParameter {
                block: *block,
                index,
            }
        })
    });
    assert!(
        params.all(|loc| {
            ir.source_map().origin_of(loc).is_none() && !covered_nodes.contains(&loc)
        })
    );
}
