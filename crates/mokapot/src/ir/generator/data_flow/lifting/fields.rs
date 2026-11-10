use ValueCategory::Category1;

use super::{FrameError, LiftContext};
use crate::{
    ir::{Operation, ValueId, expression::FieldAccess},
    jvm::references::FieldRef,
    types::field_type::ValueCategory,
};

impl<'class> LiftContext<'_, '_> {
    pub fn read_static(
        &mut self,
        field: FieldRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        self.frame
            .stack
            .push(value, field.field_type().value_category())?;
        Ok(Some(Operation::Definition {
            value,
            expr: FieldAccess::ReadStatic { field }.into(),
        }))
    }

    pub fn read_instance(
        &mut self,
        field: FieldRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let object_ref = self.frame.stack.pop(Category1)?;
        self.frame
            .stack
            .push(value, field.field_type().value_category())?;
        Ok(Some(Operation::Definition {
            value,
            expr: FieldAccess::ReadInstance { object_ref, field }.into(),
        }))
    }

    pub fn write_static(
        &mut self,
        field: FieldRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.pop_field_value(field)?;
        Ok(Some(Operation::Effect {
            expr: FieldAccess::WriteStatic { field, value }.into(),
        }))
    }

    pub fn write_instance(
        &mut self,
        field: FieldRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.pop_field_value(field)?;
        let object_ref = self.frame.stack.pop(Category1)?;
        Ok(Some(Operation::Effect {
            expr: FieldAccess::WriteInstance {
                object_ref,
                field,
                value,
            }
            .into(),
        }))
    }

    fn pop_field_value(&mut self, field: FieldRef<'_>) -> Result<ValueId, FrameError> {
        self.frame.stack.pop(field.field_type().value_category())
    }
}
