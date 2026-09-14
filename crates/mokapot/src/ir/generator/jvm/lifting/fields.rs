use crate::{
    ir::{
        expression::FieldAccess,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2},
                instruction::RegisterInstruction,
                lifting::LiftContext,
                symbolic_execution::Value,
            },
        },
    },
    jvm::references::FieldRef,
    types::field_type::{FieldType, PrimitiveType},
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn read_static(
        &mut self,
        field: &FieldRef,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        self.frame
            .push_value_of_type(&field.field_type, value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: FieldAccess::ReadStatic {
                field: field.clone(),
            }
            .into(),
        })
    }

    pub(super) fn read_instance(
        &mut self,
        field: &FieldRef,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let object_ref = self.frame.pop_value::<CATEGORY_1>()?;
        self.frame
            .push_value_of_type(&field.field_type, value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: FieldAccess::ReadInstance {
                object_ref,
                field: field.clone(),
            }
            .into(),
        })
    }

    pub(super) fn write_static(
        &mut self,
        field: &FieldRef,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.pop_field_value(field)?;
        Ok(RegisterInstruction::Effect(
            FieldAccess::WriteStatic {
                field: field.clone(),
                value,
            }
            .into(),
        ))
    }

    pub(super) fn write_instance(
        &mut self,
        field: &FieldRef,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.pop_field_value(field)?;
        let object_ref = self.frame.pop_value::<CATEGORY_1>()?;
        Ok(RegisterInstruction::Effect(
            FieldAccess::WriteInstance {
                object_ref,
                field: field.clone(),
                value,
            }
            .into(),
        ))
    }

    fn pop_field_value(&mut self, field: &FieldRef) -> Result<Value, MokaIRBuildError> {
        Ok(match field.field_type {
            FieldType::Base(PrimitiveType::Double | PrimitiveType::Long) => {
                self.frame.pop_value::<CATEGORY_2>()?
            }
            _ => self.frame.pop_value::<CATEGORY_1>()?,
        })
    }
}
