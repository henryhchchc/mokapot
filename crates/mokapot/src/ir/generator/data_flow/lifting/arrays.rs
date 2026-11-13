use ValueCategory::Category1;

use super::{FrameError, LiftContext};
use crate::{
    ir::{Operation, expression::ArrayOperation},
    types::field_type::{FieldType, ValueCategory},
};

impl<'class> LiftContext<'_, '_> {
    pub fn array_read(
        &mut self,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let index = self.frame.stack.pop(Category1)?;
        let array_ref = self.frame.stack.pop(Category1)?;
        self.frame.stack.push(value, category)?;
        Ok(Some(Operation::Definition {
            value,
            expr: ArrayOperation::Read { array_ref, index }.into(),
        }))
    }

    pub fn array_write(
        &mut self,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.frame.stack.pop(category)?;
        let index = self.frame.stack.pop(Category1)?;
        let array_ref = self.frame.stack.pop(Category1)?;
        Ok(Some(Operation::Effect {
            expr: ArrayOperation::Write {
                array_ref,
                index,
                value,
            }
            .into(),
        }))
    }

    pub fn new_array(
        &mut self,
        element_type: FieldType,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let length = self.frame.stack.pop(Category1)?;
        self.frame.stack.push(value, Category1)?;
        Ok(Some(Operation::Definition {
            value,
            expr: ArrayOperation::New {
                element_type,
                length,
            }
            .into(),
        }))
    }

    pub fn new_multi_array(
        &mut self,
        element_type: FieldType,
        dimension: u8,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let mut dimensions = (0..dimension)
            .map(|_| self.frame.stack.pop(Category1))
            .collect::<Result<Vec<_>, _>>()?;
        dimensions.reverse();
        self.frame.stack.push(value, Category1)?;
        Ok(Some(Operation::Definition {
            value,
            expr: ArrayOperation::NewMultiDim {
                element_type,
                dimensions,
            }
            .into(),
        }))
    }

    pub fn array_length(&mut self) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let array_ref = self.frame.stack.pop(Category1)?;
        self.frame.stack.push(value, Category1)?;
        Ok(Some(Operation::Definition {
            value,
            expr: ArrayOperation::Length { array_ref }.into(),
        }))
    }
}
