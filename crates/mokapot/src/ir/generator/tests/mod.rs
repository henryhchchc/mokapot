use std::ops::Range;

pub(crate) use crate::ir::test::prelude::*;
use crate::{
    ir::{
        InstructionRef, MalformedBytecode, MokaIRBuildError, MokaIRFrameError, MokaIRMethod,
        UnsupportedBytecode, ValueDefinition,
    },
    jvm::{
        Method,
        code::{ExceptionTableEntry, Instruction, ProgramCounter},
        references::ClassRef,
    },
};

/// Builds `method`, first checking the frame bookkeeping that resolution erases.
pub(crate) fn build(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    super::data_flow::verify_method(method);
    MokaIRMethod::from_method(method)
}

/// Lifts `instructions` into IR, panicking on a build failure.
pub(crate) fn lift<I, PC>(
    instructions: I,
    descriptor: &str,
    exception_table: Vec<ExceptionTableEntry>,
) -> MokaIRMethod
where
    I: IntoIterator<Item = (PC, Instruction)>,
    PC: Into<ProgramCounter>,
{
    build(&method(instructions, descriptor, exception_table)).unwrap()
}

/// The terminator whose source instruction is `pc`.
pub(crate) fn terminator_at(ir: &MokaIRMethod, pc: ProgramCounter) -> &Terminator {
    ir.source_map()
        .instructions_at(pc)
        .find_map(|it| match ir.instruction(it) {
            Some(InstructionRef::Terminator(terminator)) => Some(terminator),
            _ => None,
        })
        .expect("the source PC must map to a terminator")
}

/// The block `id` in `ir`.
pub(crate) fn block_of(ir: &MokaIRMethod, id: BlockId) -> &BasicBlock {
    ir.block(id).expect("the block belongs to its method")
}

/// The target of the successor of `block` at `index`.
pub(crate) fn successor_target(block: &BasicBlock, index: usize) -> BlockId {
    block
        .terminator
        .successors()
        .nth(index)
        .expect("the successor exists")
        .block_target()
        .expect("the successor targets a block")
}

/// The block `location` belongs to.
pub(crate) fn block_containing_instruction(
    ir: &MokaIRMethod,
    location: InstructionLocation,
) -> &BasicBlock {
    let id = match location {
        InstructionLocation::BlockParameter { block, .. }
        | InstructionLocation::Operation { block, .. }
        | InstructionLocation::Terminator { block } => block,
    };
    block_of(ir, id)
}

/// Builds an exception-table entry covering `covered_pc` that jumps to `handler_pc`.
pub(crate) fn handler(
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
pub(crate) fn malformed(method: &Method) -> (Option<ProgramCounter>, MalformedBytecode) {
    match build(method) {
        Err(MokaIRBuildError::MalformedBytecode { pc, kind }) => (pc, kind),
        other => panic!("expected malformed bytecode, got {other:?}"),
    }
}

/// Returns the frame-failure location and cause reported for `method`.
pub(crate) fn frame_failure(method: &Method) -> (Option<ProgramCounter>, MokaIRFrameError) {
    match build(method) {
        Err(MokaIRBuildError::InvalidFrame { pc, source }) => (pc, source),
        other => panic!("expected a frame failure, got {other:?}"),
    }
}

/// Returns the unsupported-bytecode location and kind reported for `method`.
pub(crate) fn unsupported(method: &Method) -> (ProgramCounter, UnsupportedBytecode) {
    match build(method) {
        Err(MokaIRBuildError::UnsupportedBytecode { pc, kind }) => (pc, kind),
        other => panic!("expected unsupported bytecode, got {other:?}"),
    }
}

mod block_arguments;
mod blocks;
mod control_flow;
mod diagnostics;
mod effects;
mod exceptions;
