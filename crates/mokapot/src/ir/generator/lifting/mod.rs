//! Lifts stack-based JVM instructions into Moka IR instructions.

mod constants;
mod control_flow;
mod locals;
mod members;
mod memory;
mod numeric;
mod objects;
mod operations;
mod stack;

use locals::{load_local, store_local};
use operations::{binary_op_math, conversion_op};

use super::{
    FrameOperand, LiftedInstruction as IR, Location, MokaIRBuildError, MokaIRGenerator,
    ProvisionalValueId,
    jvm_frame::{DUAL_SLOT, JvmStackFrame, SINGLE_SLOT},
};
use crate::{
    ir::{
        expression::{
            LiftedArrayOperation as ArrayOperation, LiftedCondition as Condition,
            LiftedConversion as Conversion, LiftedExpression as Expression,
            LiftedFieldAccess as FieldAccess, LiftedLockOperation as LockOperation,
            LiftedMathOperation as MathOperation, NaNTreatment,
        },
        generator::jvm_frame::StackOperations,
    },
    jvm::{
        ConstantValue,
        code::{Instruction, ProgramCounter, WideInstruction},
    },
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::ReturnType,
    },
};

impl MokaIRGenerator<'_> {
    pub(super) fn lift_instruction<OP>(
        &mut self,
        jvm_instruction: &Instruction,
        location: Location,
        frame: &mut JvmStackFrame<OP>,
    ) -> Result<IR<OP>, MokaIRBuildError>
    where
        OP: FrameOperand,
    {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let def = self.value_at(location)?;

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
        if let Some(instruction) = control_flow::lift(self, jvm_instruction, location, pc, frame)? {
            return Ok(instruction);
        }
        if let Some(instruction) = members::lift(jvm_instruction, def, frame)? {
            return Ok(instruction);
        }
        objects::lift(jvm_instruction, def, frame)?.ok_or(MokaIRBuildError::MalformedControlFlow)
    }
}
