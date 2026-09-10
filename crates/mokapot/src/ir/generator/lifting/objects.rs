use super::{
    ArrayOperation, Conversion, DUAL_SLOT, Expression, FieldType, FrameOperand, IR, Instruction,
    JvmStackFrame, LockOperation, MokaIRBuildError, SINGLE_SLOT, SsaValueId, WideInstruction,
    conversion_op,
};

#[expect(
    clippy::too_many_lines,
    reason = "the match is an exhaustive opcode-family dispatch"
)]
pub(super) fn lift<OP: FrameOperand>(
    jvm_instruction: &Instruction,
    def: SsaValueId,
    frame: &mut JvmStackFrame<OP>,
) -> Result<Option<IR<OP>>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use Instruction::*;

    let instruction = match jvm_instruction {
        New(class) => {
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            IR::Definition {
                value: def,
                expr: Expression::New(class.clone()),
            }
        }
        ANewArray(element_type) => {
            let count = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let array_op = ArrayOperation::New {
                element_type: element_type.clone().into(),
                length: count,
            };
            IR::Definition {
                value: def,
                expr: Expression::Array(array_op),
            }
        }
        NewArray(prim_type) => {
            let count = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let array_op = ArrayOperation::New {
                element_type: FieldType::Base(*prim_type),
                length: count,
            };
            IR::Definition {
                value: def,
                expr: Expression::Array(array_op),
            }
        }
        MultiANewArray(element_type, dimension) => {
            let counts: Vec<_> = (0..*dimension)
                .map(|_| frame.pop_value::<SINGLE_SLOT>())
                .collect::<Result<_, _>>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Array(ArrayOperation::NewMultiDim {
                element_type: element_type.clone().into(),
                dimensions: counts,
            });
            IR::Definition { value: def, expr }
        }
        ArrayLength => {
            let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Array(ArrayOperation::Length { array_ref });
            IR::Definition { value: def, expr }
        }
        AThrow => {
            let exception_ref = frame.pop_value::<SINGLE_SLOT>()?;
            IR::Throw(exception_ref)
        }
        CheckCast(target_type) => {
            conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, |value| {
                Conversion::CheckCast(value, target_type.clone())
            })?
        }
        InstanceOf(target_type) => {
            conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, |value| {
                Conversion::InstanceOf(value, target_type.clone())
            })?
        }
        MonitorEnter => {
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let monitor_op = LockOperation::Acquire(object_ref);
            let expr = Expression::Synchronization(monitor_op);
            IR::Effect(expr)
        }
        MonitorExit => {
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let monitor_op = LockOperation::Release(object_ref);
            let expr = Expression::Synchronization(monitor_op);
            IR::Effect(expr)
        }
        Wide(
            WideInstruction::ILoad(idx) | WideInstruction::FLoad(idx) | WideInstruction::ALoad(idx),
        ) => {
            let value = frame.get_local::<SINGLE_SLOT>(*idx)?;
            frame.push_value::<SINGLE_SLOT>(value)?;
            IR::Erased
        }
        Wide(WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx)) => {
            let value = frame.get_local::<DUAL_SLOT>(*idx)?;
            frame.push_value::<DUAL_SLOT>(value)?;
            IR::Erased
        }
        Wide(
            WideInstruction::IStore(idx)
            | WideInstruction::FStore(idx)
            | WideInstruction::AStore(idx),
        ) => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            frame.set_local::<SINGLE_SLOT>(*idx, value)?;
            IR::Erased
        }
        Wide(WideInstruction::LStore(idx) | WideInstruction::DStore(idx)) => {
            let value = frame.pop_value::<DUAL_SLOT>()?;
            frame.set_local::<DUAL_SLOT>(*idx, value)?;
            IR::Erased
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
