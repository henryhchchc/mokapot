use crate::ir::{
    expression::{Conversion, Expression, MathOperation},
    generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{frame::Frame, instruction::RegisterInstruction, symbolic_execution::Value},
    },
};

#[inline]
pub(super) fn lift_conversion<const OPERAND_SLOT: bool, const RESULT_SLOT: bool>(
    frame: &mut Frame<Value>,
    def: SsaValueId,
    conversion: impl FnOnce(Value) -> Conversion<Value>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let operand = frame.pop_value::<OPERAND_SLOT>()?;
    frame.push_value::<RESULT_SLOT>(def.into())?;
    Ok(RegisterInstruction::Definition {
        value: def,
        expr: Expression::Conversion(conversion(operand)),
    })
}

#[inline]
pub(super) fn lift_binary_math<const SLOT: bool>(
    frame: &mut Frame<Value>,
    def_id: SsaValueId,
    math: impl FnOnce(Value, Value) -> MathOperation<Value>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let rhs = frame.pop_value::<SLOT>()?;
    let lhs = frame.pop_value::<SLOT>()?;
    frame.push_value::<SLOT>(def_id.into())?;

    let expr = Expression::Math(math(lhs, rhs));
    Ok(RegisterInstruction::Definition {
        value: def_id,
        expr,
    })
}
