//! Lifts stack-based JVM instructions into Moka IR instructions.

mod constants;
mod control_flow;
pub(in crate::ir::generator) mod fallibility;
mod locals;
mod members;
mod memory;
mod numeric;
mod objects;
mod operations;
pub(in crate::ir::generator) mod semantics;
mod stack;

use crate::{
    ir::generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            analysis::{JvmFrameAnalyzer, OperandState},
            frame::JvmStackFrame,
            instruction::Instruction,
            normalization::Location,
        },
    },
    jvm::code::Instruction as JVM,
};
pub(super) fn lift_instruction(
    semantics: &mut JvmFrameAnalyzer<'_>,
    jvm_instruction: &JVM,
    location: Location,
    frame: &mut JvmStackFrame<OperandState>,
) -> Result<Instruction, MokaIRBuildError> {
    let pc = location
        .source_pc()
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let definition = instruction_defines_value(jvm_instruction)
        .then(|| semantics.definition_at(location))
        .transpose()?;

    if let Some(instruction) = constants::lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = memory::lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = stack::lift(jvm_instruction, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = numeric::lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = control_flow::lift(semantics, jvm_instruction, location, pc, frame)?
    {
        return Ok(instruction);
    }
    if let Some(instruction) = members::lift(jvm_instruction, definition, frame)? {
        return Ok(instruction);
    }
    objects::lift(jvm_instruction, definition, frame)?.ok_or(MokaIRBuildError::MalformedControlFlow)
}

/// Reports whether lifting an opcode creates an IR value identity.
///
/// This stays alongside the opcode-family dispatchers so values are allocated
/// before frame mutation but never for effects, control flow, or erased stack
/// operations.
const fn instruction_defines_value(instruction: &JVM) -> bool {
    constants::defines_value(instruction)
        || memory::defines_value(instruction)
        || numeric::defines_value(instruction)
        || members::defines_value(instruction)
        || objects::defines_value(instruction)
}

fn required_definition(definition: Option<SsaValueId>) -> Result<SsaValueId, MokaIRBuildError> {
    definition.ok_or(MokaIRBuildError::MalformedControlFlow)
}
