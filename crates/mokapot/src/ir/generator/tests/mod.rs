use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    ops::Range,
};

use crate::{
    ir::{
        BasicBlock, InstructionLocation, InstructionRef, MalformedBytecode, MokaIRBuildError,
        MokaIRFrameError, MokaIRMethod, Operation, Successor, Terminator, UnsupportedBytecode,
        ValueDefinition, control_flow::ControlTransfer,
    },
    jvm::{
        Method,
        code::{ExceptionTableEntry, Instruction, ProgramCounter},
        method::AccessFlags,
        references::ClassRef,
    },
};

/// Builds a `static` method for the generator to lift from the given body.
pub(super) fn method<I, PC>(
    instructions: I,
    descriptor: &str,
    exception_table: Vec<ExceptionTableEntry>,
) -> Method
where
    I: IntoIterator<Item = (PC, Instruction)>,
    PC: Into<ProgramCounter>,
{
    crate::tests::method(
        instructions,
        descriptor,
        exception_table,
        AccessFlags::PUBLIC | AccessFlags::STATIC,
    )
}

fn build(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    MokaIRMethod::from_method(method)
}

/// Builds an exception-table entry covering `covered_pc` that jumps to `handler_pc`.
pub(super) fn handler(
    covered_pc: Range<ProgramCounter>,
    handler_pc: ProgramCounter,
    catch_type: Option<ClassRef>,
) -> ExceptionTableEntry {
    ExceptionTableEntry {
        covered_pc,
        handler_pc,
        catch_type,
    }
}

/// Returns the malformed-bytecode location and kind reported for `method`.
pub(super) fn malformed(method: &Method) -> (Option<ProgramCounter>, MalformedBytecode) {
    match build(method) {
        Err(MokaIRBuildError::MalformedBytecode { pc, kind }) => (pc, kind),
        other => panic!("expected malformed bytecode, got {other:?}"),
    }
}

/// Returns the frame-failure location and cause reported for `method`.
pub(super) fn frame_failure(method: &Method) -> (Option<ProgramCounter>, MokaIRFrameError) {
    match build(method) {
        Err(MokaIRBuildError::InvalidFrame { pc, source }) => (pc, source),
        other => panic!("expected a frame failure, got {other:?}"),
    }
}

/// Returns the unsupported-bytecode location and kind reported for `method`.
pub(super) fn unsupported(method: &Method) -> (ProgramCounter, UnsupportedBytecode) {
    match build(method) {
        Err(MokaIRBuildError::UnsupportedBytecode { pc, kind }) => (pc, kind),
        other => panic!("expected unsupported bytecode, got {other:?}"),
    }
}

pub(super) fn reachable_blocks(ir: &MokaIRMethod) -> Vec<(crate::ir::BlockId, &BasicBlock)> {
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
                .filter_map(Successor::block_target),
        );
        result.push((id, block));
    }
    result
}

/// Returns the operations of `ir`'s reachable blocks.
pub(super) fn operations(ir: &MokaIRMethod) -> impl Iterator<Item = &Operation> {
    reachable_blocks(ir)
        .into_iter()
        .flat_map(|(_, it)| &it.operations)
}

/// Returns the operations of `ir`'s reachable terminators.
pub(super) fn terminator_operations(ir: &MokaIRMethod) -> impl Iterator<Item = &Operation> {
    reachable_blocks(ir)
        .into_iter()
        .filter_map(|(_, it)| it.terminator.operation())
}

/// Returns the JVM origin of `ir`'s entry terminator.
pub(super) fn entry_origin(ir: &MokaIRMethod) -> Option<ProgramCounter> {
    ir.source_map().origin_of(InstructionLocation::Terminator {
        block: ir.entry_block(),
    })
}

mod block_arguments;
mod blocks;
mod control_flow;
mod diagnostics;
mod effects;
mod exceptions;
