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
        self.define_on_stack(
            field.field_type().value_category(),
            FieldAccess::ReadStatic { field },
        )
    }

    pub fn read_instance(
        &mut self,
        field: FieldRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let object_ref = self.frame.stack.pop(Category1)?;
        self.define_on_stack(
            field.field_type().value_category(),
            FieldAccess::ReadInstance { object_ref, field },
        )
    }

    pub fn write_static(
        &mut self,
        field: FieldRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.pop_field_value(field)?;
        Ok(Some(Operation::Effect(
            FieldAccess::WriteStatic { field, value }.into(),
        )))
    }

    pub fn write_instance(
        &mut self,
        field: FieldRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.pop_field_value(field)?;
        let object_ref = self.frame.stack.pop(Category1)?;
        Ok(Some(Operation::Effect(
            FieldAccess::WriteInstance {
                object_ref,
                field,
                value,
            }
            .into(),
        )))
    }

    fn pop_field_value(&mut self, field: FieldRef<'class>) -> Result<ValueId, FrameError> {
        self.frame.stack.pop(field.field_type().value_category())
    }
}
