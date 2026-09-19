use super::super::Frame;
use super::{LiftContext, ValueCategory, definition_operation};
use crate::ir::{
    Operation, ValueId,
    expression::{Conversion, MathOperation, NaNTreatment},
    generator::error::Error,
};
use ValueCategory::{Category1, Category2};

#[inline]
pub(super) fn lift_conversion(
    frame: &mut Frame,
    value: ValueId,
    conversion: impl FnOnce(ValueId) -> Conversion,
    operand_category: ValueCategory,
    result_category: ValueCategory,
) -> Result<Option<Operation>, Error> {
    let operand = frame.stack.pop(operand_category)?;
    frame.stack.push(value, result_category)?;
    let expr = conversion(operand).into();
    Ok(Some(definition_operation(value, expr)))
}

#[inline]
pub(super) fn lift_binary_math(
    frame: &mut Frame,
    value: ValueId,
    math: impl FnOnce(ValueId, ValueId) -> MathOperation,
    category: ValueCategory,
) -> Result<Option<Operation>, Error> {
    let rhs = frame.stack.pop(category)?;
    let lhs = frame.stack.pop(category)?;
    frame.stack.push(value, category)?;

    let expr = math(lhs, rhs).into();
    Ok(Some(definition_operation(value, expr)))
}

impl LiftContext<'_, '_> {
    pub(super) fn shift_long(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let shift_amount = self.frame.stack.pop(Category1)?;
        let base = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value, Category2)?;
        let expr = operation(base, shift_amount).into();
        Ok(Some(definition_operation(value, expr)))
    }

    pub(super) fn compare_long(&mut self) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let rhs = self.frame.stack.pop(Category2)?;
        let lhs = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value, Category1)?;
        let expr = MathOperation::LongComparison(lhs, rhs).into();
        Ok(Some(definition_operation(value, expr)))
    }

    pub(super) fn compare_float(
        &mut self,
        nan_treatment: NaNTreatment,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let rhs = self.frame.stack.pop(category)?;
        let lhs = self.frame.stack.pop(category)?;
        self.frame.stack.push(value, Category1)?;
        let expr = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment).into();
        Ok(Some(definition_operation(value, expr)))
    }
}
