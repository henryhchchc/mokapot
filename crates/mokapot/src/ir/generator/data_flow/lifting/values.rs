use crate::types::class_name::ClassName;

use ValueCategory::Category1;

use super::{FrameError, LiftContext};
use crate::{
    ir::{
        Operation,
        expression::{Expression, MathOperation},
    },
    jvm::ConstantVal,
    types::field_type::ValueCategory,
};

impl<'class> LiftContext<'_, '_> {
    pub fn constant(
        &mut self,
        constant: ConstantVal<'class>,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        self.frame.stack.push(value, category)?;
        Ok(Some(Operation::Definition {
            value,
            expr: Expression::Const(constant),
        }))
    }

    pub fn increment(
        &mut self,
        idx: u16,
        constant: i32,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let base = *self.frame.locals.get(idx, Category1)?;
        self.frame.locals.set(idx, value, Category1)?;
        Ok(Some(Operation::Definition {
            value,
            expr: MathOperation::Increment(base, constant).into(),
        }))
    }

    pub fn load(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = *self.frame.locals.get(idx, category)?;
        self.frame.stack.push(value, category)?;
        Ok(None)
    }

    pub fn store(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.frame.stack.pop(category)?;
        self.frame.locals.set(idx, value, category)?;
        Ok(None)
    }

    pub fn new_object(
        &mut self,
        class_name: &'class ClassName,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        self.frame.stack.push(value, Category1)?;
        Ok(Some(Operation::Definition {
            value,
            expr: Expression::New(class_name),
        }))
    }
}
