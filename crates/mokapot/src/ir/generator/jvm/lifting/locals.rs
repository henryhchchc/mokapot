use crate::ir::generator::{
    error::MokaIRBuildError,
    jvm::{
        frame::JvmStackFrame, instruction::RegisterInstruction, symbolic_execution::SymbolicValue,
    },
};

#[inline]
pub(super) fn load_local<const SLOT: bool>(
    frame: &mut JvmStackFrame<SymbolicValue>,
    idx: u16,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let value = frame.get_local::<SLOT>(idx)?;
    if matches!(
        value,
        SymbolicValue::ReturnAddress(_) | SymbolicValue::Invalid
    ) {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }
    frame.push_value::<SLOT>(value)?;
    Ok(RegisterInstruction::Erased)
}

#[inline]
pub(super) fn store_local<const SLOT: bool>(
    frame: &mut JvmStackFrame<SymbolicValue>,
    idx: u16,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let value = frame.pop_value::<SLOT>()?;
    frame.set_local::<SLOT>(idx, value)?;
    Ok(RegisterInstruction::Erased)
}
