use super::{Condition, IR, JvmStackFrame, MokaIRBrewingError, ProgramCounter, SINGLE_SLOT};

#[inline]
pub(super) fn conditional_jump<OP: Clone + std::fmt::Display>(
    frame: &mut JvmStackFrame<OP>,
    target: ProgramCounter,
    condition: impl FnOnce(OP) -> Condition<OP>,
) -> Result<IR<OP>, MokaIRBrewingError> {
    let operand = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(IR::Jump {
        condition: Some(condition(operand)),
        target,
    })
}

#[inline]
pub(super) fn cmp_jump<OP: Clone + std::fmt::Display>(
    frame: &mut JvmStackFrame<OP>,
    target: ProgramCounter,
    condition: impl FnOnce(OP, OP) -> Condition<OP>,
) -> Result<IR<OP>, MokaIRBrewingError> {
    let rhs = frame.pop_value::<SINGLE_SLOT>()?;
    let lhs = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(IR::Jump {
        condition: Some(condition(lhs, rhs)),
        target,
    })
}
