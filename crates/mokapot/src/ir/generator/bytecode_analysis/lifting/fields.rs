use super::{LiftContext, ValueCategory, definition_operation};
use crate::{
    ir::{Operation, ValueId, expression::FieldAccess, generator::error::Error},
    jvm::references::FieldRef,
};
use ValueCategory::Category1;

impl LiftContext<'_, '_> {
    pub(super) fn read_static(&mut self, field: &FieldRef) -> Result<Option<Operation>, Error> {
        let value = self.definition_id()?;
        self.frame
            .stack
            .push(value, ValueCategory::of_field_type(&field.field_type))?;
        Ok(Some(definition_operation(
            value,
            FieldAccess::ReadStatic {
                field: field.clone(),
            }
            .into(),
        )))
    }

    pub(super) fn read_instance(&mut self, field: &FieldRef) -> Result<Option<Operation>, Error> {
        let value = self.definition_id()?;
        let object_ref = self.frame.stack.pop(Category1)?;
        self.frame
            .stack
            .push(value, ValueCategory::of_field_type(&field.field_type))?;
        Ok(Some(definition_operation(
            value,
            FieldAccess::ReadInstance {
                object_ref,
                field: field.clone(),
            }
            .into(),
        )))
    }

    pub(super) fn write_static(&mut self, field: &FieldRef) -> Result<Option<Operation>, Error> {
        let value = self.pop_field_value(field)?;
        let field = field.clone();
        Ok(Some(Operation::Effect {
            expr: FieldAccess::WriteStatic { field, value }.into(),
        }))
    }

    pub(super) fn write_instance(&mut self, field: &FieldRef) -> Result<Option<Operation>, Error> {
        let value = self.pop_field_value(field)?;
        let object_ref = self.frame.stack.pop(Category1)?;
        Ok(Some(Operation::Effect {
            expr: FieldAccess::WriteInstance {
                object_ref,
                field: field.clone(),
                value,
            }
            .into(),
        }))
    }

    fn pop_field_value(&mut self, field: &FieldRef) -> Result<ValueId, Error> {
        Ok(self
            .frame
            .stack
            .pop(ValueCategory::of_field_type(&field.field_type))?)
    }
}
