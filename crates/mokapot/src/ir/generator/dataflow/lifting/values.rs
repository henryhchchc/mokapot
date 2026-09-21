use ValueCategory::Category1;

use super::{LiftContext, ValueCategory};
use crate::{
    ir::{
        Operation,
        expression::{Expression, MathOperation},
        generator::error::Error,
    },
    jvm::{ConstantValue, references::ClassRef},
};

impl LiftContext<'_, '_> {
    pub(super) fn constant(
        &mut self,
        constant: ConstantValue,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        self.frame.stack.push(value, category)?;
        let expr = Expression::Const(constant);
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub(super) fn increment(
        &mut self,
        idx: u16,
        constant: i32,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let base = *self.frame.locals.get(idx, Category1)?;
        self.frame.locals.set(idx, value, Category1)?;
        let expr = MathOperation::Increment(base, constant).into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub(super) fn load(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = *self.frame.locals.get(idx, category)?;
        self.frame.stack.push(value, category)?;
        Ok(None)
    }

    pub(super) fn store(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.frame.stack.pop(category)?;
        self.frame.locals.set(idx, value, category)?;
        Ok(None)
    }

    pub(super) fn new_object(&mut self, class: &ClassRef) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        self.frame.stack.push(value, Category1)?;
        let expr = Expression::New(class.clone());
        Ok(Some(Operation::Definition { value, expr }))
    }
}
