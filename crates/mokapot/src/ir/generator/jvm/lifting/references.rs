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
            let definition = require_definition_id(definition)?;
            frame.push_value::<CATEGORY_1>(definition.into())?;
            RegisterInstruction::Definition {
                value: definition,
                expr: Expression::New(class.clone()),
            }
        }
        JVM::CheckCast(target_type) => {
            let definition = require_definition_id(definition)?;
            lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, definition, |value| {
                Conversion::CheckCast(value, target_type.clone())
            })?
        }
        JVM::InstanceOf(target_type) => {
            let definition = require_definition_id(definition)?;
            lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, definition, |value| {
                Conversion::InstanceOf(value, target_type.clone())
            })?
        }
        JVM::MonitorEnter => {
            let object_ref = frame.pop_value::<CATEGORY_1>()?;
            let expression = Expression::Synchronization(LockOperation::Acquire(object_ref));
            RegisterInstruction::Effect(expression)
        }
        JVM::MonitorExit => {
            let object_ref = frame.pop_value::<CATEGORY_1>()?;
            let expression = Expression::Synchronization(LockOperation::Release(object_ref));
            RegisterInstruction::Effect(expression)
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
