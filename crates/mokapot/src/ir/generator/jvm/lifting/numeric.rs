use crate::ir::{
    expression::{MathOperation, NaNTreatment},
    generator::{
        error::MokaIRBuildError,
        jvm::{
            frame::{CATEGORY_1, CATEGORY_2},
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
        let shift_amount = self.frame.pop_value::<CATEGORY_1>()?;
        let base = self.frame.pop_value::<CATEGORY_2>()?;
        self.frame.push_value::<CATEGORY_2>(value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: operation(base, shift_amount).into(),
        })
    }

    pub(super) fn compare_long(&mut self) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let rhs = self.frame.pop_value::<CATEGORY_2>()?;
        let lhs = self.frame.pop_value::<CATEGORY_2>()?;
        self.frame.push_value::<CATEGORY_1>(value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: MathOperation::LongComparison(lhs, rhs).into(),
        })
    }

    pub(super) fn compare_float<const SLOT: bool>(
        &mut self,
        nan_treatment: NaNTreatment,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let rhs = self.frame.pop_value::<SLOT>()?;
        let lhs = self.frame.pop_value::<SLOT>()?;
        self.frame.push_value::<CATEGORY_1>(value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment).into(),
        })
    }
}
