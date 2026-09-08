use super::{
    super::jvm_frame::SlotWidth, Conversion, Expression, IR, JvmStackFrame, LocalValue,
    MathOperation, MokaIRBrewingError, Operand,
};

#[inline]
pub(super) fn conversion_op<const OPERAND_SLOT: SlotWidth, const RESULT_SLOT: SlotWidth>(
    frame: &mut JvmStackFrame,
    def: LocalValue,
    conversion: impl FnOnce(Operand) -> Conversion,
) -> Result<IR, MokaIRBrewingError> {
    let operand = frame.pop_value::<OPERAND_SLOT>()?;
    frame.push_value::<RESULT_SLOT>(def.into())?;
    Ok(IR::Definition {
        value: def,
        expr: Expression::Conversion(conversion(operand)),
    })
}

#[inline]
pub(super) fn binary_op_math<const SLOT: SlotWidth>(
    frame: &mut JvmStackFrame,
    def_id: LocalValue,
    math: impl FnOnce(Operand, Operand) -> MathOperation,
) -> Result<IR, MokaIRBrewingError> {
    let rhs = frame.pop_value::<SLOT>()?;
    let lhs = frame.pop_value::<SLOT>()?;
    frame.push_value::<SLOT>(def_id.into())?;

    let expr = Expression::Math(math(lhs, rhs));
    Ok(IR::Definition {
        value: def_id,
        expr,
    })
}
