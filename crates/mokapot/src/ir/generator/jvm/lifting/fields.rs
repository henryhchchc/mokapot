use crate::{
    ir::{
        expression::FieldAccess,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2, Frame},
                instruction::RegisterInstruction,
                subroutine_expansion::Location,
                symbolic_execution::{Executor, Value},
            },
        },
    },
    jvm::code::Instruction as JVM,
    types::field_type::{FieldType, PrimitiveType},
};

impl Executor<'_> {
    pub(super) fn try_lift_fields(
        &mut self,
        jvm_instruction: &JVM,
        location: Location,
        frame: &mut Frame<Value>,
    ) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
        let instruction = match jvm_instruction {
            JVM::GetStatic(field) => {
                let value = self.definition_id_at(location)?;
                frame.push_value_of_type(&field.field_type, value.into())?;
                let field = field.clone();
                let expr = FieldAccess::ReadStatic { field }.into();
                RegisterInstruction::Definition { value, expr }
            }
            JVM::GetField(field) => {
                let value = self.definition_id_at(location)?;
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
}
