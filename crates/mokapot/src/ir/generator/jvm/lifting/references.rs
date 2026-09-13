use crate::{
    ir::{
        expression::{Conversion, Expression, LockOperation},
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{JvmStackFrame, SINGLE_SLOT},
                instruction::RegisterInstruction,
                lifting::{operations::lift_conversion, require_definition_id},
                symbolic_execution::SymbolicValue,
            },
        },
    },
    jvm::code::Instruction as JVM,
};

pub(super) const fn produces_value(instruction: &JVM) -> bool {
    matches!(
        instruction,
        JVM::New(_) | JVM::CheckCast(_) | JVM::InstanceOf(_)
    )
}

pub(super) fn try_lift(
    jvm_instruction: &JVM,
    definition: Option<SsaValueId>,
    frame: &mut JvmStackFrame<SymbolicValue>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    let instruction = match jvm_instruction {
        JVM::New(class) => {
            let definition = require_definition_id(definition)?;
            frame.push_value::<SINGLE_SLOT>(definition.into())?;
            RegisterInstruction::Definition {
                value: definition,
                expr: Expression::New(class.clone()),
            }
        }
        JVM::CheckCast(target_type) => {
            let definition = require_definition_id(definition)?;
            lift_conversion::<SINGLE_SLOT, SINGLE_SLOT>(frame, definition, |value| {
                Conversion::CheckCast(value, target_type.clone())
            })?
        }
        JVM::InstanceOf(target_type) => {
            let definition = require_definition_id(definition)?;
            lift_conversion::<SINGLE_SLOT, SINGLE_SLOT>(frame, definition, |value| {
                Conversion::InstanceOf(value, target_type.clone())
            })?
        }
        JVM::MonitorEnter => {
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let expression = Expression::Synchronization(LockOperation::Acquire(object_ref));
            RegisterInstruction::Effect(expression)
        }
        JVM::MonitorExit => {
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let expression = Expression::Synchronization(LockOperation::Release(object_ref));
            RegisterInstruction::Effect(expression)
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
