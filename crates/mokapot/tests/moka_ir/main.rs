//! Tests for generating Moka IR from compiled Java fixtures.

#![cfg(java_fixture_tests)]
#![allow(missing_docs, clippy::ignore_without_reason)]

use std::collections::{HashSet, VecDeque};

use mokapot::{
    ir::{
        BasicBlock, BlockId, InstructionLocation, InstructionRef, MokaIRMethod, Operation,
        Successor, Terminator, expression::Expression,
    },
    jvm::{Class, ConstantValue, code::ProgramCounter},
};

mod provenance;
#[path = "../support/mod.rs"]
mod support;

fn get_test_class() -> Class {
    let bytes = support::class_bytes("org/mokapot/test/TestAnalysis");
    Class::from_bytes(&bytes).unwrap()
}

fn get_test_ir() -> MokaIRMethod {
    let class = get_test_class();
    let method = class
        .methods()
        .find(|method| method.name() == "test")
        .unwrap();
    MokaIRMethod::from_method(method).unwrap()
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

#[test]
fn fixture_corpus_classes_parse_and_lift() {
    let mut class_count = 0;
    for entry in walkdir::WalkDir::new(support::classes_dir()) {
        let entry = entry.unwrap();
        if !entry.file_type().is_file()
            || entry
                .path()
                .extension()
                .is_none_or(|extension| extension != "class")
        {
            continue;
        }

        let bytes = std::fs::read(entry.path()).unwrap();
        let class = Class::from_bytes(&bytes)
            .unwrap_or_else(|error| panic!("{}: {error}", entry.path().display()));
        class_count += 1;
        for method in class.methods().filter(|method| method.body().is_some()) {
            MokaIRMethod::from_method(method).unwrap_or_else(|error| {
                panic!("{}::{}: {error}", entry.path().display(), method.name())
            });
        }
    }
    assert_ne!(class_count, 0, "the fixture corpus is empty");
}

mod semantics;
