use std::iter::repeat_n;

use ValueCategory::Category1;

use super::{FrameError, LiftContext};
use crate::{
    ir::{
        Operation,
        expression::{ArrayElementType, ArrayOperation},
    },
    types::{field_type::ValueCategory, reference_type::ReferenceType},
};

impl<'class> LiftContext<'_, '_> {
    pub fn array_read(
        &mut self,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let index = self.frame.stack.pop(Category1)?;
        let array_ref = self.frame.stack.pop(Category1)?;
        self.define_on_stack(category, ArrayOperation::Read { array_ref, index })
    }

    pub fn array_write(
        &mut self,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.frame.stack.pop(category)?;
        let index = self.frame.stack.pop(Category1)?;
        let array_ref = self.frame.stack.pop(Category1)?;
        let expr = ArrayOperation::Write {
            array_ref,
            index,
            value,
        }
        .into();
        Ok(Some(Operation::Effect(expr)))
    }

    pub fn new_array(
        &mut self,
        element_type: ArrayElementType<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let length = self.frame.stack.pop(Category1)?;
        self.define_on_stack(
            Category1,
            ArrayOperation::New {
                element_type,
                length,
            },
        )
    }

    pub fn new_multi_array(
        &mut self,
        array_type: &'class ReferenceType,
        dimension: u8,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let dimensions = self
            .frame
            .stack
            .pop_operands(repeat_n(Category1, usize::from(dimension)))?;
        self.define_on_stack(
            Category1,
            ArrayOperation::NewMultiDim {
                array_type,
                dimensions,
            },
        )
    }

    pub fn array_length(&mut self) -> Result<Option<Operation<'class>>, FrameError> {
        let array_ref = self.frame.stack.pop(Category1)?;
        self.define_on_stack(Category1, ArrayOperation::Length { array_ref })
    }
}
