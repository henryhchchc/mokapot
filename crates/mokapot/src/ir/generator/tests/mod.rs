use crate::types::class_name::ClassName;

use std::ops::Range;

pub(crate) use crate::ir::test::prelude::*;
use crate::{
    ir::{InstructionRef, MokaIRBuildError, MokaIRBuildErrorKind, MokaIRMethod},
    jvm::code::ProgramCounter,
    tests::{TestExceptionHandler, TestMethod},
};

/// Builds `method` into [`MokaIRMethod`].
pub(crate) fn build(method: &TestMethod) -> Result<MokaIRMethod, MokaIRBuildError> {
    MokaIRMethod::from_method(method.as_method())
}

/// Lifts `instructions` into IR, panicking on a build failure.
pub(crate) fn lift<I, PC>(
    instructions: I,
    descriptor: &str,
    exception_table: Vec<TestExceptionHandler>,
) -> MokaIRMethod
where
    I: IntoIterator<Item = (PC, InstructionData)>,
    PC: Into<ProgramCounter>,
{
    build(&method(instructions, descriptor, exception_table)).unwrap()
}

pub(crate) fn lift_with_pool<I, PC>(
    constant_pool: ConstantPool,
    instructions: I,
    descriptor: &str,
    exception_table: Vec<TestExceptionHandler>,
) -> MokaIRMethod
where
    I: IntoIterator<Item = (PC, InstructionData)>,
    PC: Into<ProgramCounter>,
{
    build(&method_with_pool(
        constant_pool,
        instructions,
        descriptor,
        exception_table,
    ))
    .unwrap()
}

/// The terminator whose source instruction is `pc`.
pub(crate) fn terminator_at(ir: &MokaIRMethod, pc: ProgramCounter) -> &Terminator {
    ir.source_map
        .instructions_at(pc)
        .find_map(|it| match ir.instruction(it) {
            Some(InstructionRef::Terminator(terminator)) => Some(terminator),
            _ => None,
        })
        .expect("the source PC must map to a terminator")
}

/// Builds an exception-table entry covering `covered_pc` that jumps to `handler_pc`.
pub(crate) fn handler(
    covered_pc: Range<ProgramCounter>,
    handler_pc: ProgramCounter,
    catch_type: Option<ClassName>,
) -> TestExceptionHandler {
    TestExceptionHandler {
        covered_pc,
        handler_pc,
        catch_type,
    }
}

/// Returns the diagnostic location and kind reported for `method`.
pub(crate) fn build_failure(method: &TestMethod) -> (Option<ProgramCounter>, MokaIRBuildErrorKind) {
    let error = build(method).expect_err("method should fail to build");
    (error.pc, error.kind)
}

mod block_arguments;
mod blocks;
mod control_flow;
mod effects;
mod exceptions;
