use crate::ir::{
    expression::{Conversion, MathOperation},
    generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{frame::Frame, instruction::RegisterInstruction, symbolic_execution::Value},
    },
};

#[inline]
pub(super) fn lift_conversion<const OPERAND_SLOT: bool, const RESULT_SLOT: bool>(
    frame: &mut Frame<Value>,
    value: SsaValueId,
    conversion: impl FnOnce(Value) -> Conversion<Value>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let operand = frame.pop_value::<OPERAND_SLOT>()?;
    frame.push_value::<RESULT_SLOT>(value.into())?;
    let expr = conversion(operand).into();
    Ok(RegisterInstruction::Definition { value, expr })
}

#[inline]
pub(super) fn lift_binary_math<const SLOT: bool>(
    frame: &mut Frame<Value>,
    value: SsaValueId,
    math: impl FnOnce(Value, Value) -> MathOperation<Value>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let rhs = frame.pop_value::<SLOT>()?;
    let lhs = frame.pop_value::<SLOT>()?;
    frame.push_value::<SLOT>(value.into())?;

    let expr = math(lhs, rhs).into();
    Ok(RegisterInstruction::Definition { value, expr })
}
