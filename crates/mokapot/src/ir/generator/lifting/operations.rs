use super::{
    Conversion, Expression, Instruction, JvmStackFrame, MathOperation, MokaIRBuildError, SsaValueId,
};

#[inline]
pub(super) fn conversion_op<
    const OPERAND_SLOT: bool,
    const RESULT_SLOT: bool,
    OP: Clone + From<SsaValueId>,
>(
    frame: &mut JvmStackFrame<OP>,
    def: SsaValueId,
    conversion: impl FnOnce(OP) -> Conversion<OP>,
) -> Result<Instruction<OP>, MokaIRBuildError> {
    let operand = frame.pop_value::<OPERAND_SLOT>()?;
    frame.push_value::<RESULT_SLOT>(def.into())?;
    Ok(Instruction::Definition {
        value: def,
        expr: Expression::Conversion(conversion(operand)),
    })
}

#[inline]
pub(super) fn binary_op_math<const SLOT: bool, OP: Clone + From<SsaValueId>>(
    frame: &mut JvmStackFrame<OP>,
    def_id: SsaValueId,
    math: impl FnOnce(OP, OP) -> MathOperation<OP>,
) -> Result<Instruction<OP>, MokaIRBuildError> {
    let rhs = frame.pop_value::<SLOT>()?;
    let lhs = frame.pop_value::<SLOT>()?;
    frame.push_value::<SLOT>(def_id.into())?;

    let expr = Expression::Math(math(lhs, rhs));
    Ok(Instruction::Definition {
        value: def_id,
        expr,
    })
}
