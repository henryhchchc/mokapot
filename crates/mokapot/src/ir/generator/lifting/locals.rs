use super::{
    super::{FrameOperand, jvm_frame::SlotWidth},
    IR, JvmStackFrame, MokaIRBrewingError,
};

#[inline]
pub(super) fn load_local<const SLOT: SlotWidth, OP: FrameOperand>(
    frame: &mut JvmStackFrame<OP>,
    idx: u16,
) -> Result<IR<OP>, MokaIRBrewingError> {
    let value = frame.get_local::<SLOT>(idx)?;
    if value.contains_return_address() {
        return Err(MokaIRBrewingError::MalformedControlFlow);
    }
    frame.push_value::<SLOT>(value)?;
    Ok(IR::Nop)
}

#[inline]
pub(super) fn store_local<const SLOT: SlotWidth, OP: FrameOperand>(
    frame: &mut JvmStackFrame<OP>,
    idx: u16,
) -> Result<IR<OP>, MokaIRBrewingError> {
    let value = frame.pop_value::<SLOT>()?;
    frame.set_local::<SLOT>(idx, value)?;
    Ok(IR::Nop)
}
