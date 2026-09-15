use crate::ir::{
    expression::{MathOperation, NaNTreatment},
    generator::{
        error::MokaIRBuildError,
        jvm::{
            frame::{
                ValueCategory,
                ValueCategory::{Category1, Category2},
            },
            instruction::RegisterInstruction,
            lifting::LiftContext,
            symbolic_execution::Value,
        },
    },
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn shift_long(
        &mut self,
        operation: impl FnOnce(Value, Value) -> MathOperation<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let shift_amount = self.frame.stack.pop(Category1)?;
        let base = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value.into(), Category2)?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: operation(base, shift_amount).into(),
        })
    }

    pub(super) fn compare_long(&mut self) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let rhs = self.frame.stack.pop(Category2)?;
        let lhs = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value.into(), Category1)?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: MathOperation::LongComparison(lhs, rhs).into(),
        })
    }

    pub(super) fn compare_float(
        &mut self,
        nan_treatment: NaNTreatment,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let rhs = self.frame.stack.pop(category)?;
        let lhs = self.frame.stack.pop(category)?;
        self.frame.stack.push(value.into(), Category1)?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment).into(),
        })
    }
}
