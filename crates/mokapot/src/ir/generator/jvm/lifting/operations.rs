use crate::ir::{
    expression::{Conversion, Expression, MathOperation},
    generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{analysis::OperandState, frame::JvmStackFrame, instruction::Instruction},
    },
};

#[inline]
pub(super) fn conversion_op<const OPERAND_SLOT: bool, const RESULT_SLOT: bool>(
    frame: &mut JvmStackFrame<OperandState>,
    def: SsaValueId,
    conversion: impl FnOnce(OperandState) -> Conversion<OperandState>,
) -> Result<Instruction, MokaIRBuildError> {
    let operand = frame.pop_value::<OPERAND_SLOT>()?;
    frame.push_value::<RESULT_SLOT>(def.into())?;
    Ok(Instruction::Definition {
        value: def,
        expr: Expression::Conversion(conversion(operand)),
    })
}

#[inline]
pub(super) fn binary_op_math<const SLOT: bool>(
    frame: &mut JvmStackFrame<OperandState>,
    def_id: SsaValueId,
    math: impl FnOnce(OperandState, OperandState) -> MathOperation<OperandState>,
) -> Result<Instruction, MokaIRBuildError> {
    let rhs = frame.pop_value::<SLOT>()?;
    let lhs = frame.pop_value::<SLOT>()?;
    frame.push_value::<SLOT>(def_id.into())?;

    let expr = Expression::Math(math(lhs, rhs));
    Ok(Instruction::Definition {
        value: def_id,
        expr,
    })
}
