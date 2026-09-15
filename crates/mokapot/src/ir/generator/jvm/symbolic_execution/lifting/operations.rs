use crate::ir::{
    expression::{Conversion, MathOperation, NaNTreatment},
    generator::{
        error::Error,
        identity::SsaValueId,
        jvm::{
            frame::{
                Frame, ValueCategory,
                ValueCategory::{Category1, Category2},
            },
            symbolic_execution::{RegisterInstruction, Value, lifting::Context},
        },
    },
};

#[inline]
pub(super) fn lift_conversion(
    frame: &mut Frame<Value>,
    value: SsaValueId,
    conversion: impl FnOnce(Value) -> Conversion<Value>,
    operand_category: ValueCategory,
    result_category: ValueCategory,
) -> Result<RegisterInstruction, Error> {
    let operand = frame.stack.pop(operand_category)?;
    frame.stack.push(value.into(), result_category)?;
    let expr = conversion(operand).into();
    Ok(RegisterInstruction::Definition { value, expr })
}

#[inline]
pub(super) fn lift_binary_math(
    frame: &mut Frame<Value>,
    value: SsaValueId,
    math: impl FnOnce(Value, Value) -> MathOperation<Value>,
    category: ValueCategory,
) -> Result<RegisterInstruction, Error> {
    let rhs = frame.stack.pop(category)?;
    let lhs = frame.stack.pop(category)?;
    frame.stack.push(value.into(), category)?;

    let expr = math(lhs, rhs).into();
    Ok(RegisterInstruction::Definition { value, expr })
}

impl Context<'_, '_, '_> {
    pub(super) fn shift_long(
        &mut self,
        operation: impl FnOnce(Value, Value) -> MathOperation<Value>,
    ) -> Result<RegisterInstruction, Error> {
        let value = self.definition_id()?;
        let shift_amount = self.frame.stack.pop(Category1)?;
        let base = self.frame.stack.pop(Category2)?;
        self.frame.stack.push(value.into(), Category2)?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: operation(base, shift_amount).into(),
        })
    }

    pub(super) fn compare_long(&mut self) -> Result<RegisterInstruction, Error> {
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
    ) -> Result<RegisterInstruction, Error> {
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
