//! Lifts stack-based JVM instructions into Moka IR instructions.

mod constants;
mod control_flow;
pub(in crate::ir::generator) mod fallibility;
pub(in crate::ir::generator) mod frame_operand;
mod locals;
mod members;
mod memory;
mod numeric;
mod objects;
mod operations;
pub(in crate::ir::generator) mod semantics;
mod stack;

use locals::{load_local, store_local};
use operations::{binary_op_math, conversion_op};

use super::{
    FrameOperand, Instruction, Location, MokaIRBuildError, SsaValueId,
    jvm_frame::{DUAL_SLOT, JvmStackFrame, SINGLE_SLOT},
};
use crate::{
    ir::{
        expression::{
            ArrayOperation, Condition, Conversion, Expression, FieldAccess, LockOperation,
            MathOperation, NaNTreatment,
        },
        generator::jvm_frame::StackOperations,
    },
    jvm::{
        ConstantValue,
        code::{Instruction as JVM, ProgramCounter, WideInstruction},
    },
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::ReturnType,
    },
};
use semantics::JvmSemantics;

pub(super) fn lift_instruction<OP: FrameOperand>(
    semantics: &mut impl JvmSemantics,
    jvm_instruction: &JVM,
    location: Location,
    frame: &mut JvmStackFrame<OP>,
) -> Result<Instruction<OP>, MokaIRBuildError> {
    let pc = location
        .source_pc()
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let def = semantics.definition_at(location)?;

    if let Some(instruction) = constants::lift(jvm_instruction, def, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = memory::lift(jvm_instruction, def, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = stack::lift(jvm_instruction, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = numeric::lift(jvm_instruction, def, frame)? {
        return Ok(instruction);
    }
    if let Some(instruction) = control_flow::lift(semantics, jvm_instruction, location, pc, frame)?
    {
        return Ok(instruction);
    }
    if let Some(instruction) = members::lift(jvm_instruction, def, frame)? {
        return Ok(instruction);
    }
    objects::lift(jvm_instruction, def, frame)?.ok_or(MokaIRBuildError::MalformedControlFlow)
}
