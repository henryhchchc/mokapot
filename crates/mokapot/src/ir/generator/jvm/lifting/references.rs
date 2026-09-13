use crate::{
    ir::{
        expression::{Conversion, Expression, LockOperation},
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{CATEGORY_1, Frame},
                instruction::RegisterInstruction,
                lifting::{operations::lift_conversion, require_definition_id},
                symbolic_execution::Value,
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
    frame: &mut Frame<Value>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    let instruction = match jvm_instruction {
        JVM::New(class) => {
            let value = require_definition_id(definition)?;
            frame.push_value::<CATEGORY_1>(value.into())?;
            let expr = Expression::New(class.clone());
            RegisterInstruction::Definition { value, expr }
        }
        JVM::CheckCast(target_type) => {
            let value = require_definition_id(definition)?;
            lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, value, |value| {
                Conversion::CheckCast(value, target_type.clone())
            })?
        }
        JVM::InstanceOf(target_type) => {
            let value = require_definition_id(definition)?;
            lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, value, |value| {
                Conversion::InstanceOf(value, target_type.clone())
            })?
        }
        JVM::MonitorEnter => {
            let object_ref = frame.pop_value::<CATEGORY_1>()?;
            let lock_op = LockOperation::Acquire(object_ref).into();
            RegisterInstruction::Effect(lock_op)
        }
        JVM::MonitorExit => {
            let object_ref = frame.pop_value::<CATEGORY_1>()?;
            let lock_op = LockOperation::Release(object_ref).into();
            RegisterInstruction::Effect(lock_op)
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
