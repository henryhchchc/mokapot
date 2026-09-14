//! Lifts stack-based JVM instructions into register-based instructions.

mod arrays;
mod calls;
mod constants;
mod control_flow;
pub(super) mod fallibility;
mod fields;
mod locals;
mod miscellaneous;
mod numeric;
mod operations;
mod references;
mod stack;
pub(super) mod successors;

use crate::{
    ir::generator::{
        error::MokaIRBuildError,
        jvm::{
            frame::Frame,
            instruction::RegisterInstruction,
            subroutine_expansion::Location,
            symbolic_execution::{Executor, Value},
        },
    },
    jvm::code::Instruction as JVM,
};

impl Executor<'_> {
    pub(super) fn lift_register_instruction(
        &mut self,
        jvm_instruction: &JVM,
        location: Location,
        frame: &mut Frame<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;

        if let Some(instruction) = self.try_lift_constants(jvm_instruction, location, frame)? {
            return Ok(instruction);
        }
        if let Some(instruction) = self.try_lift_locals(jvm_instruction, location, frame)? {
            return Ok(instruction);
        }
        if let Some(instruction) = self.try_lift_arrays(jvm_instruction, location, frame)? {
            return Ok(instruction);
        }
        if let Some(instruction) = Self::try_lift_stack(jvm_instruction, frame)? {
            return Ok(instruction);
        }
        if let Some(instruction) = self.try_lift_numeric(jvm_instruction, location, frame)? {
            return Ok(instruction);
        }
        if let Some(instruction) =
            self.try_lift_control_flow(jvm_instruction, location, pc, frame)?
        {
            return Ok(instruction);
        }
        if let Some(instruction) = self.try_lift_fields(jvm_instruction, location, frame)? {
            return Ok(instruction);
        }
        if let Some(instruction) = self.try_lift_calls(jvm_instruction, location, frame)? {
            return Ok(instruction);
        }
        if let Some(instruction) = Self::try_lift_miscellaneous(jvm_instruction) {
            return Ok(instruction);
        }
        self.try_lift_references(jvm_instruction, location, frame)?
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }
}
