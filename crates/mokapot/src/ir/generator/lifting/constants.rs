use super::{
    ConstantValue, DUAL_SLOT, Expression, FrameOperand, IR, Instruction, JvmStackFrame,
    MokaIRBuildError, SINGLE_SLOT, SsaValueId,
};

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
        Nop | Breakpoint | ImpDep1 | ImpDep2 => IR::Erased,
        AConstNull => {
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Const(ConstantValue::Null);
            IR::Definition { value: def, expr }
        }
        IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let int_value = i32::from(jvm_instruction.opcode()) - 3;
            let expr = Expression::Const(ConstantValue::Integer(int_value));
            IR::Definition { value: def, expr }
        }
        LConst0 | LConst1 => {
            let value = def.into();
            frame.push_value::<DUAL_SLOT>(value)?;
            let long_value = i64::from(jvm_instruction.opcode()) - 9;
            let expr = Expression::Const(ConstantValue::Long(long_value));
            IR::Definition { value: def, expr }
        }
        FConst0 | FConst1 | FConst2 => {
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let float_value = f32::from(jvm_instruction.opcode()) - 11.0;
            let expr = Expression::Const(ConstantValue::Float(float_value));
            IR::Definition { value: def, expr }
        }
        DConst0 | DConst1 => {
            let value = def.into();
            frame.push_value::<DUAL_SLOT>(value)?;
            let double_value = f64::from(jvm_instruction.opcode()) - 14.0;
            let expr = Expression::Const(ConstantValue::Double(double_value));
            IR::Definition { value: def, expr }
        }
        BiPush(value) => {
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Const(ConstantValue::Integer(i32::from(*value)));
            IR::Definition { value: def, expr }
        }
        SiPush(value) => {
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Const(ConstantValue::Integer(i32::from(*value)));
            IR::Definition { value: def, expr }
        }
        Ldc(value) | LdcW(value) => {
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Const(value.clone());
            IR::Definition { value: def, expr }
        }
        Ldc2W(value) => {
            frame.push_value::<DUAL_SLOT>(def.into())?;
            let expr = Expression::Const(value.clone());
            IR::Definition { value: def, expr }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
