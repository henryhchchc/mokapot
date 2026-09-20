use super::{LiftContext, ValueCategory, definition_operation};
use crate::{
    ir::{Operation, expression::ArrayOperation, generator::error::Error},
    types::field_type::FieldType,
};
use ValueCategory::Category1;

impl LiftContext<'_, '_> {
    pub(super) fn array_read(
        &mut self,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let index = self.frame.stack.pop(Category1)?;
        let array_ref = self.frame.stack.pop(Category1)?;
        self.frame.stack.push(value, category)?;
        let expr = ArrayOperation::Read { array_ref, index }.into();
        Ok(Some(definition_operation(value, expr)))
    }

    pub(super) fn array_write(
        &mut self,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.frame.stack.pop(category)?;
        let index = self.frame.stack.pop(Category1)?;
        let array_ref = self.frame.stack.pop(Category1)?;
        let expr = ArrayOperation::Write {
            array_ref,
            index,
            value,
        }
        .into();
        Ok(Some(Operation::Effect { expr }))
    }

    pub(super) fn new_array(
        &mut self,
        element_type: FieldType,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let length = self.frame.stack.pop(Category1)?;
        self.frame.stack.push(value, Category1)?;
        let expr = ArrayOperation::New {
            element_type,
            length,
        }
        .into();
        Ok(Some(definition_operation(value, expr)))
    }

    pub(super) fn new_multi_array(
        &mut self,
        element_type: FieldType,
        dimension: u8,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let dimensions = (0..dimension)
            .map(|_| self.frame.stack.pop(Category1))
            .collect::<Result<_, _>>()?;
        self.frame.stack.push(value, Category1)?;
        let expr = ArrayOperation::NewMultiDim {
            element_type,
            dimensions,
        }
        .into();
        Ok(Some(definition_operation(value, expr)))
    }

    pub(super) fn array_length(&mut self) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let array_ref = self.frame.stack.pop(Category1)?;
        self.frame.stack.push(value, Category1)?;
        let expr = ArrayOperation::Length { array_ref }.into();
        Ok(Some(definition_operation(value, expr)))
    }
}
