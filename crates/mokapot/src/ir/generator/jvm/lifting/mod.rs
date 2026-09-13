//! Lifts stack-based JVM instructions into register-based instructions.

mod arrays;
mod calls;
mod constants;
mod control_flow;
pub(super) mod fallibility;
mod fields;
mod locals;
mod numeric;
mod operations;
mod references;
mod stack;
pub(super) mod successors;

use crate::{
    ir::generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            frame::JvmStackFrame,
            instruction::RegisterInstruction,
            normalization::Location,
            symbolic_execution::{JvmSymbolicExecutor, SymbolicValue},
        },
    },
    jvm::code::Instruction as JVM,
};
pub(super) fn lift_register_instruction(
    executor: &mut JvmSymbolicExecutor<'_>,
    jvm_instruction: &JVM,
    location: Location,
    frame: &mut JvmStackFrame<SymbolicValue>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let pc = location
        .source_pc()
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let definition = produces_register_value(jvm_instruction)
        .then(|| executor.definition_at(location))
        .transpose()?;

    if let Some(instruction) = constants::try_lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = locals::try_lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = arrays::try_lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = stack::try_lift(jvm_instruction, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = numeric::try_lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) =
        control_flow::try_lift(executor, jvm_instruction, location, pc, frame)?
    {
        return Ok(instruction);
    }
    if let Some(instruction) = fields::try_lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = calls::try_lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    references::try_lift(jvm_instruction, definition, frame)?
        .ok_or(MokaIRBuildError::MalformedControlFlow)
}

/// Reports whether lifting an opcode creates an IR value identity.
///
/// This stays alongside the opcode-family dispatchers so values are allocated
/// before frame mutation but never for effects, control flow, or erased stack
/// operations.
const fn produces_register_value(instruction: &JVM) -> bool {
    constants::produces_value(instruction)
        || locals::produces_value(instruction)
        || arrays::produces_value(instruction)
        || numeric::produces_value(instruction)
        || fields::produces_value(instruction)
        || calls::produces_value(instruction)
        || references::produces_value(instruction)
}

fn require_definition_id(definition: Option<SsaValueId>) -> Result<SsaValueId, MokaIRBuildError> {
    definition.ok_or(MokaIRBuildError::MalformedControlFlow)
}
