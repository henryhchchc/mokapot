use ValueCategory::{Category1, Category2};

use super::{FrameError, LiftContext};
use crate::{
    ir::{
        Operation, ValueId,
        expression::{Conversion, MathOperation, NaNTreatment},
    },
    types::field_type::ValueCategory,
};

impl<'class> LiftContext<'_, '_> {
    pub fn shift_long(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let shift_amount = self.frame.stack.pop(Category1)?;
        let base = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value, Category2)?;
        Ok(Some(Operation::Definition {
            value,
            expr: operation(base, shift_amount).into(),
        }))
    }

    pub fn compare_long(&mut self) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let rhs = self.frame.stack.pop(Category2)?;
        let lhs = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value, Category1)?;
        Ok(Some(Operation::Definition {
            value,
            expr: MathOperation::LongComparison(lhs, rhs).into(),
        }))
    }

    pub fn compare_float(
        &mut self,
        nan_treatment: NaNTreatment,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let rhs = self.frame.stack.pop(category)?;
        let lhs = self.frame.stack.pop(category)?;
        self.frame.stack.push(value, Category1)?;
        Ok(Some(Operation::Definition {
            value,
            expr: MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment).into(),
        }))
    }

    pub fn binary(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let rhs = self.frame.stack.pop(category)?;
        let lhs = self.frame.stack.pop(category)?;
        self.frame.stack.push(value, category)?;

        Ok(Some(Operation::Definition {
            value,
            expr: operation(lhs, rhs).into(),
        }))
    }

    pub fn conversion(
        &mut self,
        conversion: impl FnOnce(ValueId) -> Conversion<'class>,
        operand_category: ValueCategory,
        result_category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let operand = self.frame.stack.pop(operand_category)?;
        self.frame.stack.push(value, result_category)?;
        Ok(Some(Operation::Definition {
            value,
            expr: conversion(operand).into(),
        }))
    }

    pub fn unary(
        &mut self,
        operation: impl FnOnce(ValueId) -> MathOperation,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let operand = self.frame.stack.pop(category)?;
        self.frame.stack.push(value, category)?;
        Ok(Some(Operation::Definition {
            value,
            expr: operation(operand).into(),
        }))
    }
}
