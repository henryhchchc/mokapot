use crate::{
    ir::{
        expression::{Expression, MathOperation},
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{ValueCategory, ValueCategory::Category1},
                symbolic_execution::{RegisterInstruction, Value, lifting::Context},
            },
        },
    },
    jvm::{ConstantValue, references::ClassRef},
};

impl Context<'_, '_, '_> {
    pub(super) fn constant(
        &mut self,
        constant: ConstantValue,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        self.frame.stack.push(value.into(), category)?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: Expression::Const(constant),
        })
    }

    pub(super) fn increment(
        &mut self,
        idx: u16,
        constant: i32,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let base = *self.frame.locals.get(idx, Category1)?;
        self.frame.locals.set(idx, value.into(), Category1)?;
        let expr = MathOperation::Increment(base, constant).into();
        Ok(RegisterInstruction::Definition { value, expr })
    }

    pub(super) fn load(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = *self.frame.locals.get(idx, category)?;
        if matches!(value, Value::ReturnAddress(_) | Value::Invalid) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        self.frame.stack.push(value, category)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn load_unchecked(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = *self.frame.locals.get(idx, category)?;
        self.frame.stack.push(value, category)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn store(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.frame.stack.pop(category)?;
        self.frame.locals.set(idx, value, category)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn new_object(
        &mut self,
        class: &ClassRef,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        self.frame.stack.push(value.into(), Category1)?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: Expression::New(class.clone()),
        })
    }
}
