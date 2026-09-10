#[allow(
    clippy::wildcard_imports,
    reason = "opcode-family lifters share the private lifting vocabulary"
)]
use super::*;

pub(super) fn lift<OP: FrameOperand>(
    jvm_instruction: &Instruction,
    def: ProvisionalValueId,
    frame: &mut JvmStackFrame<OP>,
) -> Result<Option<IR<OP>>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use Instruction::*;

    let instruction = match jvm_instruction {
        ILoad(idx) | FLoad(idx) | ALoad(idx) => {
            load_local::<SINGLE_SLOT, _>(frame, u16::from(*idx))?
        }
        LLoad(idx) | DLoad(idx) => load_local::<DUAL_SLOT, _>(frame, (*idx).into())?,
        ILoad0 | FLoad0 | ALoad0 => load_local::<SINGLE_SLOT, _>(frame, 0)?,
        ILoad1 | FLoad1 | ALoad1 => load_local::<SINGLE_SLOT, _>(frame, 1)?,
        ILoad2 | FLoad2 | ALoad2 => load_local::<SINGLE_SLOT, _>(frame, 2)?,
        ILoad3 | FLoad3 | ALoad3 => load_local::<SINGLE_SLOT, _>(frame, 3)?,
        LLoad0 | DLoad0 => load_local::<DUAL_SLOT, _>(frame, 0)?,
        LLoad1 | DLoad1 => load_local::<DUAL_SLOT, _>(frame, 1)?,
        LLoad2 | DLoad2 => load_local::<DUAL_SLOT, _>(frame, 2)?,
        LLoad3 | DLoad3 => load_local::<DUAL_SLOT, _>(frame, 3)?,
        IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
            let index = frame.pop_value::<SINGLE_SLOT>()?;
            let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let array_op = ArrayOperation::Read { array_ref, index };

            frame.push_value::<SINGLE_SLOT>(def.into())?;
            IR::Definition {
                value: def,
                expr: Expression::Array(array_op),
            }
        }
        LALoad | DALoad => {
            let index = frame.pop_value::<SINGLE_SLOT>()?;
            let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let array_op = ArrayOperation::Read { array_ref, index };
            frame.push_value::<DUAL_SLOT>(def.into())?;
            IR::Definition {
                value: def,
                expr: Expression::Array(array_op),
            }
        }
        IStore(idx) | FStore(idx) | AStore(idx) => {
            store_local::<SINGLE_SLOT, _>(frame, u16::from(*idx))?
        }
        LStore(idx) | DStore(idx) => store_local::<DUAL_SLOT, _>(frame, u16::from(*idx))?,
        IStore0 | FStore0 | AStore0 => store_local::<SINGLE_SLOT, _>(frame, 0)?,
        IStore1 | FStore1 | AStore1 => store_local::<SINGLE_SLOT, _>(frame, 1)?,
        IStore2 | FStore2 | AStore2 => store_local::<SINGLE_SLOT, _>(frame, 2)?,
        IStore3 | FStore3 | AStore3 => store_local::<SINGLE_SLOT, _>(frame, 3)?,
        LStore0 | DStore0 => store_local::<DUAL_SLOT, _>(frame, 0)?,
        LStore1 | DStore1 => store_local::<DUAL_SLOT, _>(frame, 1)?,
        LStore2 | DStore2 => store_local::<DUAL_SLOT, _>(frame, 2)?,
        LStore3 | DStore3 => store_local::<DUAL_SLOT, _>(frame, 3)?,
        IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            let index = frame.pop_value::<SINGLE_SLOT>()?;
            let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let array_op = ArrayOperation::Write {
                array_ref,
                index,
                value,
            };

            IR::Effect(Expression::Array(array_op))
        }
        LAStore | DAStore => {
            let value = frame.pop_value::<DUAL_SLOT>()?;
            let index = frame.pop_value::<SINGLE_SLOT>()?;
            let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let array_op = ArrayOperation::Write {
                array_ref,
                index,
                value,
            };
            IR::Effect(Expression::Array(array_op))
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
