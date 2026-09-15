use crate::{
    ir::{
        expression::{Expression, MathOperation},
        generator::{
            bytecode_analysis::{
                RegisterInstruction, Value,
                jvm::{ValueCategory, ValueCategory::Category1},
                lifting::Context,
            },
            error::{Error, MalformedBytecode},
        },
    },
    jvm::{ConstantValue, references::ClassRef},
};

impl Context<'_, '_, '_> {
    pub(super) fn constant(
        &mut self,
        constant: ConstantValue,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, Error> {
        let value = self.definition_id()?;
        self.frame.stack.push(value.into(), category)?;
        let expr = Expression::Const(constant);
        Ok(RegisterInstruction::Definition { value, expr })
    }

    pub(super) fn increment(
        &mut self,
        idx: u16,
        constant: i32,
    ) -> Result<RegisterInstruction, Error> {
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
    ) -> Result<RegisterInstruction, Error> {
        let value = *self.frame.locals.get(idx, category)?;
        if matches!(value, Value::ReturnAddress(_) | Value::Invalid) {
            let pc = self.addr.source_pc().ok_or_else(|| {
                Error::internal("a local-variable load has no source instruction")
            })?;
            return Err(Error::malformed(
                Some(pc),
                MalformedBytecode::InvalidFrameValue,
            ));
        }
        self.frame.stack.push(value, category)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn load_unchecked(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, Error> {
        let value = *self.frame.locals.get(idx, category)?;
        self.frame.stack.push(value, category)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn store(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, Error> {
        let value = self.frame.stack.pop(category)?;
        self.frame.locals.set(idx, value, category)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn new_object(&mut self, class: &ClassRef) -> Result<RegisterInstruction, Error> {
        let value = self.definition_id()?;
        self.frame.stack.push(value.into(), Category1)?;
        let expr = Expression::New(class.clone());
        Ok(RegisterInstruction::Definition { value, expr })
    }
}
