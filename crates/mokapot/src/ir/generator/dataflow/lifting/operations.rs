use ValueCategory::{Category1, Category2};

use super::LiftContext;
use crate::{
    ir::{
        Operation, ValueId,
        expression::{Conversion, MathOperation, NaNTreatment},
        generator::error::Error,
    },
    types::field_type::ValueCategory,
};

impl LiftContext<'_, '_> {
    pub fn shift_long(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let shift_amount = self.frame.stack.pop(Category1)?;
        let base = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value, Category2)?;
        let expr = operation(base, shift_amount).into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub fn compare_long(&mut self) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let rhs = self.frame.stack.pop(Category2)?;
        let lhs = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value, Category1)?;
        let expr = MathOperation::LongComparison(lhs, rhs).into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub fn compare_float(
        &mut self,
        nan_treatment: NaNTreatment,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let rhs = self.frame.stack.pop(category)?;
        let lhs = self.frame.stack.pop(category)?;
        self.frame.stack.push(value, Category1)?;
        let expr = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment).into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub fn binary(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let rhs = self.frame.stack.pop(category)?;
        let lhs = self.frame.stack.pop(category)?;
        self.frame.stack.push(value, category)?;

        let expr = operation(lhs, rhs).into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub fn conversion(
        &mut self,
        conversion: impl FnOnce(ValueId) -> Conversion,
        operand_category: ValueCategory,
        result_category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let operand = self.frame.stack.pop(operand_category)?;
        self.frame.stack.push(value, result_category)?;
        let expr = conversion(operand).into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub fn unary(
        &mut self,
        operation: impl FnOnce(ValueId) -> MathOperation,
        category: ValueCategory,
    ) -> Result<Option<Operation>, Error> {
        let value = self.definition_id();
        let operand = self.frame.stack.pop(category)?;
        self.frame.stack.push(value, category)?;
        let expr = operation(operand).into();
        Ok(Some(Operation::Definition { value, expr }))
    }
}
