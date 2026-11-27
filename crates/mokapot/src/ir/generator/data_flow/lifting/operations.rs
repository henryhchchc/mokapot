use ValueCategory::{Category1, Category2};

use super::{FrameError, LiftContext};
use crate::{
    ir::{
        Operation, ValueId,
        expression::{Expression, MathOperation, NaNTreatment},
    },
    types::field_type::ValueCategory,
};

impl<'class> LiftContext<'_, '_> {
    pub fn shift_long(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        self.binary_with_categories(operation, [Category2, Category1], Category2)
    }

    pub fn compare_long(&mut self) -> Result<Option<Operation<'class>>, FrameError> {
        self.binary_with_categories(MathOperation::LongComparison, [Category2; 2], Category1)
    }

    pub fn compare_float(
        &mut self,
        nan_treatment: NaNTreatment,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        self.binary_with_categories(
            |lhs, rhs| MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment),
            [category; 2],
            Category1,
        )
    }

    pub fn binary(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        self.binary_with_categories(operation, [category; 2], category)
    }

    fn binary_with_categories(
        &mut self,
        operation: impl FnOnce(ValueId, ValueId) -> MathOperation,
        [lhs_category, rhs_category]: [ValueCategory; 2],
        result_category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let rhs = self.frame.stack.pop(rhs_category)?;
        let lhs = self.frame.stack.pop(lhs_category)?;
        self.define_on_stack(result_category, operation(lhs, rhs))
    }

    pub fn unary<E>(
        &mut self,
        operation: impl FnOnce(ValueId) -> E,
        operand_category: ValueCategory,
        result_category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError>
    where
        E: Into<Expression<'class>>,
    {
        let operand = self.frame.stack.pop(operand_category)?;
        self.define_on_stack(result_category, operation(operand))
    }
}
