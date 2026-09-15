use crate::{
    ir::{
        expression::FieldAccess,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::ValueCategory::{self, Category1},
                instruction::RegisterInstruction,
                lifting::LiftContext,
                symbolic_execution::Value,
            },
        },
    },
    jvm::references::FieldRef,
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn read_static(
        &mut self,
        field: &FieldRef,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        self.frame.stack.push(
            value.into(),
            ValueCategory::of_field_type(&field.field_type),
        )?;
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
        let object_ref = self.frame.stack.pop(Category1)?;
        self.frame.stack.push(
            value.into(),
            ValueCategory::of_field_type(&field.field_type),
        )?;
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
        let object_ref = self.frame.stack.pop(Category1)?;
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
        Ok(self
            .frame
            .stack
            .pop(ValueCategory::of_field_type(&field.field_type))?)
    }
}
