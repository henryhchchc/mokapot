use super::{
    super::jvm_frame::SlotWidth, Conversion, Expression, IR, JvmStackFrame, MathOperation,
    MokaIRBuildError, SsaValueId,
};

#[inline]
pub(super) fn conversion_op<
    const OPERAND_SLOT: SlotWidth,
    const RESULT_SLOT: SlotWidth,
    OP: Clone + From<SsaValueId>,
>(
    frame: &mut JvmStackFrame<OP>,
    def: SsaValueId,
    conversion: impl FnOnce(OP) -> Conversion<OP>,
) -> Result<IR<OP>, MokaIRBuildError> {
    let operand = frame.pop_value::<OPERAND_SLOT>()?;
    frame.push_value::<RESULT_SLOT>(def.into())?;
    Ok(IR::Definition {
        value: def,
        expr: Expression::Conversion(conversion(operand)),
    })
}

#[inline]
pub(super) fn binary_op_math<const SLOT: SlotWidth, OP: Clone + From<SsaValueId>>(
    frame: &mut JvmStackFrame<OP>,
    def_id: SsaValueId,
    math: impl FnOnce(OP, OP) -> MathOperation<OP>,
) -> Result<IR<OP>, MokaIRBuildError> {
    let rhs = frame.pop_value::<SLOT>()?;
    let lhs = frame.pop_value::<SLOT>()?;
    frame.push_value::<SLOT>(def_id.into())?;

    let expr = Expression::Math(math(lhs, rhs));
    Ok(IR::Definition {
        value: def_id,
        expr,
    })
}
