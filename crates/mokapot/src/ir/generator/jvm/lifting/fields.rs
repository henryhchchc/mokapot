use crate::{
    ir::{
        expression::{Expression, FieldAccess},
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
            let definition = require_definition_id(definition)?;
            frame.push_value_of_type(&field.field_type, definition.into())?;
            let field_op = FieldAccess::ReadStatic {
                field: field.clone(),
            };
            RegisterInstruction::Definition {
                value: definition,
                expr: Expression::Field(field_op),
            }
        }
        JVM::GetField(field) => {
            let definition = require_definition_id(definition)?;
            let object_ref = frame.pop_value::<CATEGORY_1>()?;
            frame.push_value_of_type(&field.field_type, definition.into())?;
            let field_op = FieldAccess::ReadInstance {
                object_ref,
                field: field.clone(),
            };
            RegisterInstruction::Definition {
                value: definition,
                expr: Expression::Field(field_op),
            }
        }
        JVM::PutStatic(field) => {
            use PrimitiveType::{Double, Long};
            let value = if let FieldType::Base(Double | Long) = field.field_type {
                frame.pop_value::<CATEGORY_2>()
            } else {
                frame.pop_value::<CATEGORY_1>()
            }?;
            let field_op = FieldAccess::WriteStatic {
                field: field.clone(),
                value,
            };
            RegisterInstruction::Effect(Expression::Field(field_op))
        }
        JVM::PutField(field) => {
            use PrimitiveType::{Double, Long};
            let value = if let FieldType::Base(Double | Long) = field.field_type {
                frame.pop_value::<CATEGORY_2>()
            } else {
                frame.pop_value::<CATEGORY_1>()
            }?;
            let object_ref = frame.pop_value::<CATEGORY_1>()?;
            let field_op = FieldAccess::WriteInstance {
                object_ref,
                field: field.clone(),
                value,
            };
            RegisterInstruction::Effect(Expression::Field(field_op))
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
