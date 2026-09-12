use crate::ir::generator::{
    error::MokaIRBuildError,
    jvm::{analysis::OperandState, frame::JvmStackFrame, instruction::Instruction},
};

#[inline]
pub(super) fn load_local<const SLOT: bool>(
    frame: &mut JvmStackFrame<OperandState>,
    idx: u16,
) -> Result<Instruction, MokaIRBuildError> {
    let value = frame.get_local::<SLOT>(idx)?;
    if matches!(
        value,
        OperandState::ReturnAddress(_) | OperandState::Invalid
    ) {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }
    frame.push_value::<SLOT>(value)?;
    Ok(Instruction::Erased)
}

#[inline]
pub(super) fn store_local<const SLOT: bool>(
    frame: &mut JvmStackFrame<OperandState>,
    idx: u16,
) -> Result<Instruction, MokaIRBuildError> {
    let value = frame.pop_value::<SLOT>()?;
    frame.set_local::<SLOT>(idx, value)?;
    Ok(Instruction::Erased)
}
