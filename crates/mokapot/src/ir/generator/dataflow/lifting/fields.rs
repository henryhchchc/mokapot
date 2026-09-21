use ValueCategory::Category1;

use super::LiftContext;
use crate::{
    ir::{Operation, ValueId, expression::FieldAccess, generator::error::Error},
    jvm::references::FieldRef,
    types::field_type::ValueCategory,
};

impl LiftContext<'_, '_> {
    pub fn read_static(&mut self, field: &FieldRef) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        self.frame
            .stack
            .push(value, field.field_type.value_category())?;
        let expr = FieldAccess::ReadStatic {
            field: field.clone(),
        }
        .into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub fn read_instance(&mut self, field: &FieldRef) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let object_ref = self.frame.stack.pop(Category1)?;
        self.frame
            .stack
            .push(value, field.field_type.value_category())?;
        let expr = FieldAccess::ReadInstance {
            object_ref,
            field: field.clone(),
        }
        .into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub fn write_static(&mut self, field: &FieldRef) -> Result<Option<Operation>, Error> {
        let value = self.pop_field_value(field)?;
        let field = field.clone();
        Ok(Some(Operation::Effect {
            expr: FieldAccess::WriteStatic { field, value }.into(),
        }))
    }

    pub fn write_instance(&mut self, field: &FieldRef) -> Result<Option<Operation>, Error> {
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
        Ok(self.frame.stack.pop(field.field_type.value_category())?)
    }
}
