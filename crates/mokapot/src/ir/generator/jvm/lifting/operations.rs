use crate::ir::{
    expression::{Conversion, MathOperation},
    generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            frame::{Frame, ValueCategory},
            instruction::RegisterInstruction,
            symbolic_execution::Value,
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
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let operand = frame.operand_stack.pop(operand_category)?;
    frame.operand_stack.push(value.into(), result_category)?;
    let expr = conversion(operand).into();
    Ok(RegisterInstruction::Definition { value, expr })
}

#[inline]
pub(super) fn lift_binary_math(
    frame: &mut Frame<Value>,
    value: SsaValueId,
    math: impl FnOnce(Value, Value) -> MathOperation<Value>,
    category: ValueCategory,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let rhs = frame.operand_stack.pop(category)?;
    let lhs = frame.operand_stack.pop(category)?;
    frame.operand_stack.push(value.into(), category)?;

    let expr = math(lhs, rhs).into();
    Ok(RegisterInstruction::Definition { value, expr })
}
