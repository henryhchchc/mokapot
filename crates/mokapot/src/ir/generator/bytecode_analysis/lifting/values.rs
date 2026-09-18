use super::definition_operation;
use crate::{
    ir::{
        OperationKind,
        expression::{Expression, MathOperation},
        generator::{
            bytecode_analysis::{
                jvm::{ValueCategory, ValueCategory::Category1},
                lifting::Context,
            },
            error::Error,
        },
    },
    jvm::{ConstantValue, references::ClassRef},
};

impl Context<'_, '_, '_> {
    pub(super) fn constant(
        &mut self,
        constant: ConstantValue,
        category: ValueCategory,
    ) -> Result<Option<OperationKind>, Error> {
        let value = self.definition_id()?;
        self.frame.stack.push(value, category)?;
        let expr = Expression::Const(constant);
        Ok(Some(definition_operation(value, expr)))
    }

    pub(super) fn increment(
        &mut self,
        idx: u16,
        constant: i32,
    ) -> Result<Option<OperationKind>, Error> {
        let value = self.definition_id()?;
        let base = *self.frame.locals.get(idx, Category1)?;
        self.frame.locals.set(idx, value, Category1)?;
        let expr = MathOperation::Increment(base, constant).into();
        Ok(Some(definition_operation(value, expr)))
    }

    pub(super) fn load(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<Option<OperationKind>, Error> {
        let value = *self.frame.locals.get(idx, category)?;
        self.frame.stack.push(value, category)?;
        Ok(None)
    }

    pub(super) fn store(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<Option<OperationKind>, Error> {
        let value = self.frame.stack.pop(category)?;
        self.frame.locals.set(idx, value, category)?;
        Ok(None)
    }

    pub(super) fn new_object(&mut self, class: &ClassRef) -> Result<Option<OperationKind>, Error> {
        let value = self.definition_id()?;
        self.frame.stack.push(value, Category1)?;
        let expr = Expression::New(class.clone());
        Ok(Some(definition_operation(value, expr)))
    }
}
