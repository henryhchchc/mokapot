use crate::{
    ir::{
        expression::{Expression, MathOperation},
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::CATEGORY_1, instruction::RegisterInstruction, lifting::LiftContext,
                symbolic_execution::Value,
            },
        },
    },
    jvm::{ConstantValue, references::ClassRef},
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn constant<const SLOT: bool>(
        &mut self,
        constant: ConstantValue,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        self.frame.push_value::<SLOT>(value.into())?;
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
        let base = self.frame.get_local::<CATEGORY_1>(idx)?;
        self.frame.set_local::<CATEGORY_1>(idx, value.into())?;
        let expr = MathOperation::Increment(base, constant).into();
        Ok(RegisterInstruction::Definition { value, expr })
    }

    pub(super) fn load<const SLOT: bool>(
        &mut self,
        idx: u16,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.frame.get_local::<SLOT>(idx)?;
        if matches!(value, Value::ReturnAddress(_) | Value::Invalid) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        self.frame.push_value::<SLOT>(value)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn load_unchecked<const SLOT: bool>(
        &mut self,
        idx: u16,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.frame.get_local::<SLOT>(idx)?;
        self.frame.push_value::<SLOT>(value)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn store<const SLOT: bool>(
        &mut self,
        idx: u16,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.frame.pop_value::<SLOT>()?;
        self.frame.set_local::<SLOT>(idx, value)?;
        Ok(RegisterInstruction::Erased)
    }

    pub(super) fn new_object(
        &mut self,
        class: &ClassRef,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        self.frame.push_value::<CATEGORY_1>(value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: Expression::New(class.clone()),
        })
    }
}
