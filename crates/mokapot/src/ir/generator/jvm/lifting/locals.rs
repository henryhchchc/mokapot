use crate::{
    ir::{
        expression::MathOperation,
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2, Frame},
                instruction::RegisterInstruction,
                lifting::require_definition_id,
                symbolic_execution::Value,
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
    frame: &mut Frame<Value>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    use JVM::{
        ALoad, ALoad0, ALoad1, ALoad2, ALoad3, AStore, AStore0, AStore1, AStore2, AStore3, DLoad,
        DLoad0, DLoad1, DLoad2, DLoad3, DStore, DStore0, DStore1, DStore2, DStore3, FLoad, FLoad0,
        FLoad1, FLoad2, FLoad3, FStore, FStore0, FStore1, FStore2, FStore3, IInc, ILoad, ILoad0,
        ILoad1, ILoad2, ILoad3, IStore, IStore0, IStore1, IStore2, IStore3, LLoad, LLoad0, LLoad1,
        LLoad2, LLoad3, LStore, LStore0, LStore1, LStore2, LStore3, Wide,
    };

    let instruction = match jvm_instruction {
        ILoad(idx) | FLoad(idx) | ALoad(idx) => load_local::<CATEGORY_1>(frame, u16::from(*idx))?,
        LLoad(idx) | DLoad(idx) => load_local::<CATEGORY_2>(frame, (*idx).into())?,
        ILoad0 | FLoad0 | ALoad0 => load_local::<CATEGORY_1>(frame, 0)?,
        ILoad1 | FLoad1 | ALoad1 => load_local::<CATEGORY_1>(frame, 1)?,
        ILoad2 | FLoad2 | ALoad2 => load_local::<CATEGORY_1>(frame, 2)?,
        ILoad3 | FLoad3 | ALoad3 => load_local::<CATEGORY_1>(frame, 3)?,
        LLoad0 | DLoad0 => load_local::<CATEGORY_2>(frame, 0)?,
        LLoad1 | DLoad1 => load_local::<CATEGORY_2>(frame, 1)?,
        LLoad2 | DLoad2 => load_local::<CATEGORY_2>(frame, 2)?,
        LLoad3 | DLoad3 => load_local::<CATEGORY_2>(frame, 3)?,
        IStore(idx) | FStore(idx) | AStore(idx) => {
            store_local::<CATEGORY_1>(frame, u16::from(*idx))?
        }
        LStore(idx) | DStore(idx) => store_local::<CATEGORY_2>(frame, u16::from(*idx))?,
        IStore0 | FStore0 | AStore0 => store_local::<CATEGORY_1>(frame, 0)?,
        IStore1 | FStore1 | AStore1 => store_local::<CATEGORY_1>(frame, 1)?,
        IStore2 | FStore2 | AStore2 => store_local::<CATEGORY_1>(frame, 2)?,
        IStore3 | FStore3 | AStore3 => store_local::<CATEGORY_1>(frame, 3)?,
        LStore0 | DStore0 => store_local::<CATEGORY_2>(frame, 0)?,
        LStore1 | DStore1 => store_local::<CATEGORY_2>(frame, 1)?,
        LStore2 | DStore2 => store_local::<CATEGORY_2>(frame, 2)?,
        LStore3 | DStore3 => store_local::<CATEGORY_2>(frame, 3)?,
        IInc(idx, constant) => increment_local(frame, (*idx).into(), *constant, definition)?,
        Wide(WideInstruction::IInc(idx, constant)) => {
            increment_local(frame, *idx, *constant, definition)?
        }
        Wide(
            WideInstruction::ILoad(idx) | WideInstruction::FLoad(idx) | WideInstruction::ALoad(idx),
        ) => {
            let value = frame.get_local::<CATEGORY_1>(*idx)?;
            frame.push_value::<CATEGORY_1>(value)?;
            RegisterInstruction::Erased
        }
        Wide(WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx)) => {
            let value = frame.get_local::<CATEGORY_2>(*idx)?;
            frame.push_value::<CATEGORY_2>(value)?;
            RegisterInstruction::Erased
        }
        Wide(
            WideInstruction::IStore(idx)
            | WideInstruction::FStore(idx)
            | WideInstruction::AStore(idx),
        ) => {
            let value = frame.pop_value::<CATEGORY_1>()?;
            frame.set_local::<CATEGORY_1>(*idx, value)?;
            RegisterInstruction::Erased
        }
        Wide(WideInstruction::LStore(idx) | WideInstruction::DStore(idx)) => {
            let value = frame.pop_value::<CATEGORY_2>()?;
            frame.set_local::<CATEGORY_2>(*idx, value)?;
            RegisterInstruction::Erased
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}

fn increment_local(
    frame: &mut Frame<Value>,
    idx: u16,
    constant: i32,
    definition: Option<SsaValueId>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let value = require_definition_id(definition)?;
    let base = frame.get_local::<CATEGORY_1>(idx)?;
    frame.set_local::<CATEGORY_1>(idx, value.into())?;
    let expr = MathOperation::Increment(base, constant).into();
    Ok(RegisterInstruction::Definition { value, expr })
}

#[inline]
fn load_local<const SLOT: bool>(
    frame: &mut Frame<Value>,
    idx: u16,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let value = frame.get_local::<SLOT>(idx)?;
    if matches!(value, Value::ReturnAddress(_) | Value::Invalid) {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }
    frame.push_value::<SLOT>(value)?;
    Ok(RegisterInstruction::Erased)
}

#[inline]
fn store_local<const SLOT: bool>(
    frame: &mut Frame<Value>,
    idx: u16,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let value = frame.pop_value::<SLOT>()?;
    frame.set_local::<SLOT>(idx, value)?;
    Ok(RegisterInstruction::Erased)
}
