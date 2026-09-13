use crate::{
    ir::{
        expression::{Expression, MathOperation},
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{DUAL_SLOT, JvmStackFrame, SINGLE_SLOT},
                instruction::RegisterInstruction,
                lifting::require_definition_id,
                symbolic_execution::SymbolicValue,
            },
        },
    },
    jvm::code::{Instruction as JVM, WideInstruction},
};

pub(super) const fn produces_value(instruction: &JVM) -> bool {
    matches!(
        instruction,
        JVM::IInc(_, _) | JVM::Wide(WideInstruction::IInc(_, _))
    )
}

pub(super) fn try_lift(
    jvm_instruction: &JVM,
    definition: Option<SsaValueId>,
    frame: &mut JvmStackFrame<SymbolicValue>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use JVM::*;

    let instruction = match jvm_instruction {
        ILoad(idx) | FLoad(idx) | ALoad(idx) => load_local::<SINGLE_SLOT>(frame, u16::from(*idx))?,
        LLoad(idx) | DLoad(idx) => load_local::<DUAL_SLOT>(frame, (*idx).into())?,
        ILoad0 | FLoad0 | ALoad0 => load_local::<SINGLE_SLOT>(frame, 0)?,
        ILoad1 | FLoad1 | ALoad1 => load_local::<SINGLE_SLOT>(frame, 1)?,
        ILoad2 | FLoad2 | ALoad2 => load_local::<SINGLE_SLOT>(frame, 2)?,
        ILoad3 | FLoad3 | ALoad3 => load_local::<SINGLE_SLOT>(frame, 3)?,
        LLoad0 | DLoad0 => load_local::<DUAL_SLOT>(frame, 0)?,
        LLoad1 | DLoad1 => load_local::<DUAL_SLOT>(frame, 1)?,
        LLoad2 | DLoad2 => load_local::<DUAL_SLOT>(frame, 2)?,
        LLoad3 | DLoad3 => load_local::<DUAL_SLOT>(frame, 3)?,
        IStore(idx) | FStore(idx) | AStore(idx) => {
            store_local::<SINGLE_SLOT>(frame, u16::from(*idx))?
        }
        LStore(idx) | DStore(idx) => store_local::<DUAL_SLOT>(frame, u16::from(*idx))?,
        IStore0 | FStore0 | AStore0 => store_local::<SINGLE_SLOT>(frame, 0)?,
        IStore1 | FStore1 | AStore1 => store_local::<SINGLE_SLOT>(frame, 1)?,
        IStore2 | FStore2 | AStore2 => store_local::<SINGLE_SLOT>(frame, 2)?,
        IStore3 | FStore3 | AStore3 => store_local::<SINGLE_SLOT>(frame, 3)?,
        LStore0 | DStore0 => store_local::<DUAL_SLOT>(frame, 0)?,
        LStore1 | DStore1 => store_local::<DUAL_SLOT>(frame, 1)?,
        LStore2 | DStore2 => store_local::<DUAL_SLOT>(frame, 2)?,
        LStore3 | DStore3 => store_local::<DUAL_SLOT>(frame, 3)?,
        IInc(idx, constant) => increment_local(frame, (*idx).into(), *constant, definition)?,
        Wide(WideInstruction::IInc(idx, constant)) => {
            increment_local(frame, *idx, *constant, definition)?
        }
        Wide(
            WideInstruction::ILoad(idx) | WideInstruction::FLoad(idx) | WideInstruction::ALoad(idx),
        ) => {
            let value = frame.get_local::<SINGLE_SLOT>(*idx)?;
            frame.push_value::<SINGLE_SLOT>(value)?;
            RegisterInstruction::Erased
        }
        Wide(WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx)) => {
            let value = frame.get_local::<DUAL_SLOT>(*idx)?;
            frame.push_value::<DUAL_SLOT>(value)?;
            RegisterInstruction::Erased
        }
        Wide(
            WideInstruction::IStore(idx)
            | WideInstruction::FStore(idx)
            | WideInstruction::AStore(idx),
        ) => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            frame.set_local::<SINGLE_SLOT>(*idx, value)?;
            RegisterInstruction::Erased
        }
        Wide(WideInstruction::LStore(idx) | WideInstruction::DStore(idx)) => {
            let value = frame.pop_value::<DUAL_SLOT>()?;
            frame.set_local::<DUAL_SLOT>(*idx, value)?;
            RegisterInstruction::Erased
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}

fn increment_local(
    frame: &mut JvmStackFrame<SymbolicValue>,
    idx: u16,
    constant: i32,
    definition: Option<SsaValueId>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let definition = require_definition_id(definition)?;
    let base = frame.get_local::<SINGLE_SLOT>(idx)?;
    frame.set_local::<SINGLE_SLOT>(idx, definition.into())?;
    let operation = MathOperation::Increment(base, constant);
    Ok(RegisterInstruction::Definition {
        value: definition,
        expr: Expression::Math(operation),
    })
}

#[inline]
fn load_local<const SLOT: bool>(
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
fn store_local<const SLOT: bool>(
    frame: &mut JvmStackFrame<SymbolicValue>,
    idx: u16,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let value = frame.pop_value::<SLOT>()?;
    frame.set_local::<SLOT>(idx, value)?;
    Ok(RegisterInstruction::Erased)
}
