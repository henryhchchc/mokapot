use crate::ir::generator::{
    error::MokaIRBuildError,
    jvm::{frame::JvmStackFrame, instruction::Instruction, lifting::frame_operand::FrameOperand},
};

#[inline]
pub(super) fn load_local<const SLOT: bool, OP: FrameOperand>(
    frame: &mut JvmStackFrame<OP>,
    idx: u16,
) -> Result<Instruction<OP>, MokaIRBuildError> {
    let value = frame.get_local::<SLOT>(idx)?;
    if value.contains_return_address() {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }
    frame.push_value::<SLOT>(value)?;
    Ok(Instruction::Erased)
}

#[inline]
pub(super) fn store_local<const SLOT: bool, OP: FrameOperand>(
    frame: &mut JvmStackFrame<OP>,
    idx: u16,
) -> Result<Instruction<OP>, MokaIRBuildError> {
    let value = frame.pop_value::<SLOT>()?;
    frame.set_local::<SLOT>(idx, value)?;
    Ok(Instruction::Erased)
}
