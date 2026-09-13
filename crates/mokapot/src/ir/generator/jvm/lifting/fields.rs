use crate::{
    ir::{
        expression::{Expression, FieldAccess},
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{DUAL_SLOT, JvmStackFrame, SINGLE_SLOT},
                instruction::RegisterInstruction,
                lifting::require_definition_id,
                symbolic_execution::SymbolicValue,
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
    frame: &mut JvmStackFrame<SymbolicValue>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    let instruction = match jvm_instruction {
        JVM::GetStatic(field) => {
            let definition = require_definition_id(definition)?;
            frame.typed_push(&field.field_type, definition.into())?;
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
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            frame.typed_push(&field.field_type, definition.into())?;
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
                frame.pop_value::<DUAL_SLOT>()
            } else {
                frame.pop_value::<SINGLE_SLOT>()
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
                frame.pop_value::<DUAL_SLOT>()
            } else {
                frame.pop_value::<SINGLE_SLOT>()
            }?;
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
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
