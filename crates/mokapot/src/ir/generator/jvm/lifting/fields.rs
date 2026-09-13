use crate::{
    ir::{
        expression::FieldAccess,
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2, Frame},
                instruction::RegisterInstruction,
                lifting::require_definition_id,
                symbolic_execution::Value,
            },
        },
    },
    jvm::code::Instruction as JVM,
    types::field_type::{FieldType, PrimitiveType},
};

pub(super) const fn produces_value(instruction: &JVM) -> bool {
    matches!(instruction, JVM::GetStatic(_) | JVM::GetField(_))
}

pub(super) fn try_lift(
    jvm_instruction: &JVM,
    definition: Option<SsaValueId>,
    frame: &mut Frame<Value>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    let instruction = match jvm_instruction {
        JVM::GetStatic(field) => {
            let value = require_definition_id(definition)?;
            frame.push_value_of_type(&field.field_type, value.into())?;
            let field = field.clone();
            let expr = FieldAccess::ReadStatic { field }.into();
            RegisterInstruction::Definition { value, expr }
        }
        JVM::GetField(field) => {
            let value = require_definition_id(definition)?;
            let object_ref = frame.pop_value::<CATEGORY_1>()?;
            frame.push_value_of_type(&field.field_type, value.into())?;
            let field = field.clone();
            let expr = FieldAccess::ReadInstance { object_ref, field }.into();
            RegisterInstruction::Definition { value, expr }
        }
        JVM::PutStatic(field) => {
            use PrimitiveType::{Double, Long};
            let value = match field.field_type {
                FieldType::Base(Double | Long) => frame.pop_value::<CATEGORY_2>(),
                _ => frame.pop_value::<CATEGORY_1>(),
            }?;
            let field = field.clone();
            let field_op = FieldAccess::WriteStatic { field, value }.into();
            RegisterInstruction::Effect(field_op)
        }
        JVM::PutField(field) => {
            use PrimitiveType::{Double, Long};
            let value = match field.field_type {
                FieldType::Base(Double | Long) => frame.pop_value::<CATEGORY_2>(),
                _ => frame.pop_value::<CATEGORY_1>(),
            }?;
            let field_op = FieldAccess::WriteInstance {
                object_ref: frame.pop_value::<CATEGORY_1>()?,
                field: field.clone(),
                value,
            }
            .into();
            RegisterInstruction::Effect(field_op)
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
