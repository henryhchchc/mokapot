use crate::ir::{
    expression::{Conversion, MathOperation, NaNTreatment},
    generator::{
        bytecode_analysis::{
            FrameValue, LiftedEffect,
            jvm::{
                Frame, ValueCategory,
                ValueCategory::{Category1, Category2},
            },
            lifting::Context,
        },
        error::Error,
        identity::SsaValueId,
    },
};

#[inline]
pub(super) fn lift_conversion(
    frame: &mut Frame<FrameValue>,
    value: SsaValueId,
    conversion: impl FnOnce(FrameValue) -> Conversion<FrameValue>,
    operand_category: ValueCategory,
    result_category: ValueCategory,
) -> Result<LiftedEffect, Error> {
    let operand = frame.stack.pop(operand_category)?;
    frame.stack.push(value.into(), result_category)?;
    let expr = conversion(operand).into();
    Ok(LiftedEffect::Definition { value, expr })
}

#[inline]
pub(super) fn lift_binary_math(
    frame: &mut Frame<FrameValue>,
    value: SsaValueId,
    math: impl FnOnce(FrameValue, FrameValue) -> MathOperation<FrameValue>,
    category: ValueCategory,
) -> Result<LiftedEffect, Error> {
    let rhs = frame.stack.pop(category)?;
    let lhs = frame.stack.pop(category)?;
    frame.stack.push(value.into(), category)?;

    let expr = math(lhs, rhs).into();
    Ok(LiftedEffect::Definition { value, expr })
}

impl Context<'_, '_, '_> {
    pub(super) fn shift_long(
        &mut self,
        operation: impl FnOnce(FrameValue, FrameValue) -> MathOperation<FrameValue>,
    ) -> Result<LiftedEffect, Error> {
        let value = self.definition_id()?;
        let shift_amount = self.frame.stack.pop(Category1)?;
        let base = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value.into(), Category2)?;
        let expr = operation(base, shift_amount).into();
        Ok(LiftedEffect::Definition { value, expr })
    }

    pub(super) fn compare_long(&mut self) -> Result<LiftedEffect, Error> {
        let value = self.definition_id()?;
        let rhs = self.frame.stack.pop(Category2)?;
        let lhs = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value.into(), Category1)?;
        let expr = MathOperation::LongComparison(lhs, rhs).into();
        Ok(LiftedEffect::Definition { value, expr })
    }

    pub(super) fn compare_float(
        &mut self,
        nan_treatment: NaNTreatment,
        category: ValueCategory,
    ) -> Result<LiftedEffect, Error> {
        let value = self.definition_id()?;
        let rhs = self.frame.stack.pop(category)?;
        let lhs = self.frame.stack.pop(category)?;
        self.frame.stack.push(value.into(), Category1)?;
        let expr = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment).into();
        Ok(LiftedEffect::Definition { value, expr })
    }
}
