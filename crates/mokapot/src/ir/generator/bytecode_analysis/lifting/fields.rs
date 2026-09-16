use super::definition_operation;
use crate::{
    ir::{
        OperationKind,
        expression::FieldAccess,
        generator::{
            bytecode_analysis::{
                FrameValue,
                jvm::ValueCategory::{self, Category1},
                lifting::Context,
            },
            error::Error,
        },
    },
    jvm::references::FieldRef,
};

impl Context<'_, '_, '_> {
    pub(super) fn read_static(
        &mut self,
        field: &FieldRef,
    ) -> Result<Option<OperationKind<FrameValue>>, Error> {
        let value = self.definition_id()?;
        self.frame.stack.push(
            value.into(),
            ValueCategory::of_field_type(&field.field_type),
        )?;
        Ok(Some(definition_operation(
            value,
            FieldAccess::ReadStatic {
                field: field.clone(),
            }
            .into(),
        )))
    }

    pub(super) fn read_instance(
        &mut self,
        field: &FieldRef,
    ) -> Result<Option<OperationKind<FrameValue>>, Error> {
        let value = self.definition_id()?;
        let object_ref = self.frame.stack.pop(Category1)?;
        self.frame.stack.push(
            value.into(),
            ValueCategory::of_field_type(&field.field_type),
        )?;
        Ok(Some(definition_operation(
            value,
            FieldAccess::ReadInstance {
                object_ref,
                field: field.clone(),
            }
            .into(),
        )))
    }

    pub(super) fn write_static(
        &mut self,
        field: &FieldRef,
    ) -> Result<Option<OperationKind<FrameValue>>, Error> {
        let value = self.pop_field_value(field)?;
        let field = field.clone();
        Ok(Some(OperationKind::Effect {
            expr: FieldAccess::WriteStatic { field, value }.into(),
        }))
    }

    pub(super) fn write_instance(
        &mut self,
        field: &FieldRef,
    ) -> Result<Option<OperationKind<FrameValue>>, Error> {
        let value = self.pop_field_value(field)?;
        let object_ref = self.frame.stack.pop(Category1)?;
        Ok(Some(OperationKind::Effect {
            expr: FieldAccess::WriteInstance {
                object_ref,
                field: field.clone(),
                value,
            }
            .into(),
        }))
    }

    fn pop_field_value(&mut self, field: &FieldRef) -> Result<FrameValue, Error> {
        Ok(self
            .frame
            .stack
            .pop(ValueCategory::of_field_type(&field.field_type))?)
    }
}
