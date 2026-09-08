use super::{
    Condition, IR, JvmStackFrame, MokaIRBrewingError, Operand, ProgramCounter, SINGLE_SLOT,
};

#[inline]
pub(super) fn conditional_jump(
    frame: &mut JvmStackFrame,
    target: ProgramCounter,
    condition: impl FnOnce(Operand) -> Condition,
) -> Result<IR, MokaIRBrewingError> {
    let operand = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(IR::Jump {
        condition: Some(condition(operand)),
        target,
    })
}

#[inline]
pub(super) fn cmp_jump(
    frame: &mut JvmStackFrame,
    target: ProgramCounter,
    condition: impl FnOnce(Operand, Operand) -> Condition,
) -> Result<IR, MokaIRBrewingError> {
    let rhs = frame.pop_value::<SINGLE_SLOT>()?;
    let lhs = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(IR::Jump {
        condition: Some(condition(lhs, rhs)),
        target,
    })
}
