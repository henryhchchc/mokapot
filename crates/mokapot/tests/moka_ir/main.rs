#![allow(missing_docs, clippy::ignore_without_reason)]

use std::collections::{HashSet, VecDeque};

use mokapot::{
    ir::{
        BasicBlock, BlockId, InstructionLocation, InstructionRef, MokaIRMethod, Operation,
        Successor, Terminator, expression::Expression,
    },
    jvm::{Class, ConstantValue, JavaString, Method, code::ProgramCounter},
};

mod provenance;
mod source_map;
mod ssa;

fn get_test_class() -> Class {
    let mut bytes = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/mokapot/java_classes/org/mokapot/test/TestAnalysis.class"
    ))
    .as_slice();
    Class::from_reader(&mut bytes).unwrap()
}

fn get_test_method() -> Method {
    get_test_class()
        .methods
        .into_iter()
        .find(|method| method.name == "test")
        .unwrap()
}

fn terminator(method: &MokaIRMethod, location: InstructionLocation) -> Option<&Terminator> {
    match method.instruction(location) {
        Some(InstructionRef::Terminator(terminator)) => Some(terminator),
        Some(InstructionRef::BlockParameter(_) | InstructionRef::Operation(_)) | None => None,
    }
}

fn reachable_blocks(ir: &MokaIRMethod) -> Vec<(BlockId, &BasicBlock)> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut pending = VecDeque::from([ir.entry.block]);
    while let Some(id) = pending.pop_front() {
        if !visited.insert(id) {
            continue;
        }
        let block = ir.block(id).expect("a successor belongs to its method");
        pending.extend(
            block
                .terminator
                .successors()
                .filter_map(Successor::block_target),
        );
        result.push((id, block));
    }
    result
}

#[cfg(integration_test)]
fn live_locations(ir: &MokaIRMethod) -> Vec<InstructionLocation> {
    let mut locations = Vec::new();
    for (block_id, block) in reachable_blocks(ir) {
        for (index, _) in block.parameters.iter().enumerate() {
            locations.push(InstructionLocation::BlockParameter {
                block: block_id,
                index,
            });
        }
        for (index, _) in block.operations.iter().enumerate() {
            locations.push(InstructionLocation::Operation {
                block: block_id,
                index,
            });
        }
        locations.push(InstructionLocation::Terminator { block: block_id });
    }
    locations
}

/// Every method with a body in the fixture corpus (`OUT_DIR/mokapot/java_classes`,
/// compiled by `build.rs`).
#[cfg(integration_test)]
fn corpus_methods() -> Vec<(String, Method)> {
    let mut directories = vec![std::path::Path::new(env!("OUT_DIR")).join("mokapot/java_classes")];
    let mut class_files = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory).expect("the corpus directory is readable") {
            let path = entry.expect("a corpus entry is readable").path();
            if path.is_dir() {
                directories.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "class")
            {
                class_files.push(path);
            }
        }
    }
    class_files.sort();
    let mut methods = Vec::new();
    for path in class_files {
        let bytes = std::fs::read(&path).expect("a fixture class is readable");
        let mut reader: &[u8] = &bytes;
        let class = Class::from_reader(&mut reader)
            .unwrap_or_else(|error| panic!("Failed to parse {}: {error}", path.display()));
        for method in class.methods {
            if method.body.is_some() {
                let label = format!("{}::{}", path.display(), method.name);
                methods.push((label, method));
            }
        }
    }
    assert!(!methods.is_empty(), "the fixture corpus is empty");
    methods
}

#[cfg(integration_test)]
fn corpus_ir() -> Vec<(String, MokaIRMethod)> {
    corpus_methods()
        .into_iter()
        .map(|(label, method)| {
            let ir = MokaIRMethod::from_method(&method)
                .unwrap_or_else(|error| panic!("Failed to build {label}: {error}"));
            (label, ir)
        })
        .collect()
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn load_test_method() {
    get_test_method();
}
