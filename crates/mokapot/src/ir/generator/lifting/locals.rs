use super::{super::jvm_frame::SlotWidth, IR, JvmStackFrame, MokaIRBrewingError};

#[inline]
pub(super) fn load_local<const SLOT: SlotWidth>(
    frame: &mut JvmStackFrame,
    idx: u16,
) -> Result<IR, MokaIRBrewingError> {
    let value = frame.get_local::<SLOT>(idx)?;
    frame.push_value::<SLOT>(value)?;
    Ok(IR::Nop)
}

#[inline]
pub(super) fn store_local<const SLOT: SlotWidth>(
    frame: &mut JvmStackFrame,
    idx: u16,
) -> Result<IR, MokaIRBrewingError> {
    let value = frame.pop_value::<SLOT>()?;
    frame.set_local::<SLOT>(idx, value)?;
    Ok(IR::Nop)
}
