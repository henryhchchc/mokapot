use super::{
    ArrayOperation, Conversion, DUAL_SLOT, Expression, FieldType, FrameOperand, Instruction, JVM,
    JvmStackFrame, LockOperation, MokaIRBuildError, SINGLE_SLOT, SsaValueId, WideInstruction,
    conversion_op, required_definition,
};

pub(super) const fn defines_value(instruction: &JVM) -> bool {
    matches!(
        instruction,
        JVM::New(_)
            | JVM::ANewArray(_)
            | JVM::NewArray(_)
            | JVM::MultiANewArray(_, _)
            | JVM::ArrayLength
            | JVM::CheckCast(_)
            | JVM::InstanceOf(_)
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "the match is an exhaustive opcode-family dispatch"
)]
pub(super) fn lift<OP: FrameOperand>(
    jvm_instruction: &JVM,
    definition: Option<SsaValueId>,
    frame: &mut JvmStackFrame<OP>,
) -> Result<Option<Instruction<OP>>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use JVM::*;

    let instruction = match jvm_instruction {
        New(class) => {
            let def = required_definition(definition)?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            Instruction::Definition {
                value: def,
                expr: Expression::New(class.clone()),
            }
        }
        ANewArray(element_type) => {
            let def = required_definition(definition)?;
            let count = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let array_op = ArrayOperation::New {
                element_type: element_type.clone().into(),
                length: count,
            };
            Instruction::Definition {
                value: def,
                expr: Expression::Array(array_op),
            }
        }
        NewArray(prim_type) => {
            let def = required_definition(definition)?;
            let count = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let array_op = ArrayOperation::New {
                element_type: FieldType::Base(*prim_type),
                length: count,
            };
            Instruction::Definition {
                value: def,
                expr: Expression::Array(array_op),
            }
        }
        MultiANewArray(element_type, dimension) => {
            let def = required_definition(definition)?;
            let counts: Vec<_> = (0..*dimension)
                .map(|_| frame.pop_value::<SINGLE_SLOT>())
                .collect::<Result<_, _>>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Array(ArrayOperation::NewMultiDim {
                element_type: element_type.clone().into(),
                dimensions: counts,
            });
            Instruction::Definition { value: def, expr }
        }
        ArrayLength => {
            let def = required_definition(definition)?;
            let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Array(ArrayOperation::Length { array_ref });
            Instruction::Definition { value: def, expr }
        }
        AThrow => {
            let exception_ref = frame.pop_value::<SINGLE_SLOT>()?;
            Instruction::Throw(exception_ref)
        }
        CheckCast(target_type) => {
            let def = required_definition(definition)?;
            conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, |value| {
                Conversion::CheckCast(value, target_type.clone())
            })?
        }
        InstanceOf(target_type) => {
            let def = required_definition(definition)?;
            conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, |value| {
                Conversion::InstanceOf(value, target_type.clone())
            })?
        }
        MonitorEnter => {
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let monitor_op = LockOperation::Acquire(object_ref);
            let expr = Expression::Synchronization(monitor_op);
            Instruction::Effect(expr)
        }
        MonitorExit => {
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let monitor_op = LockOperation::Release(object_ref);
            let expr = Expression::Synchronization(monitor_op);
            Instruction::Effect(expr)
        }
        Wide(
            WideInstruction::ILoad(idx) | WideInstruction::FLoad(idx) | WideInstruction::ALoad(idx),
        ) => {
            let value = frame.get_local::<SINGLE_SLOT>(*idx)?;
            frame.push_value::<SINGLE_SLOT>(value)?;
            Instruction::Erased
        }
        Wide(WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx)) => {
            let value = frame.get_local::<DUAL_SLOT>(*idx)?;
            frame.push_value::<DUAL_SLOT>(value)?;
            Instruction::Erased
        }
        Wide(
            WideInstruction::IStore(idx)
            | WideInstruction::FStore(idx)
            | WideInstruction::AStore(idx),
        ) => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            frame.set_local::<SINGLE_SLOT>(*idx, value)?;
            Instruction::Erased
        }
        Wide(WideInstruction::LStore(idx) | WideInstruction::DStore(idx)) => {
            let value = frame.pop_value::<DUAL_SLOT>()?;
            frame.set_local::<DUAL_SLOT>(*idx, value)?;
            Instruction::Erased
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
