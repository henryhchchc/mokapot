use super::{
    ConstantValue, DUAL_SLOT, Expression, FrameOperand, Instruction, JVM, JvmStackFrame,
    MokaIRBuildError, SINGLE_SLOT, SsaValueId, required_definition,
};

pub(super) const fn defines_value(instruction: &JVM) -> bool {
    matches!(
        instruction,
        JVM::AConstNull
            | JVM::IConstM1
            | JVM::IConst0
            | JVM::IConst1
            | JVM::IConst2
            | JVM::IConst3
            | JVM::IConst4
            | JVM::IConst5
            | JVM::LConst0
            | JVM::LConst1
            | JVM::FConst0
            | JVM::FConst1
            | JVM::FConst2
            | JVM::DConst0
            | JVM::DConst1
            | JVM::BiPush(_)
            | JVM::SiPush(_)
            | JVM::Ldc(_)
            | JVM::LdcW(_)
            | JVM::Ldc2W(_)
    )
}

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
        Nop | Breakpoint | ImpDep1 | ImpDep2 => Instruction::Erased,
        AConstNull => {
            let def = required_definition(definition)?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Const(ConstantValue::Null);
            Instruction::Definition { value: def, expr }
        }
        IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
            let def = required_definition(definition)?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let int_value = i32::from(jvm_instruction.opcode()) - 3;
            let expr = Expression::Const(ConstantValue::Integer(int_value));
            Instruction::Definition { value: def, expr }
        }
        LConst0 | LConst1 => {
            let def = required_definition(definition)?;
            let value = def.into();
            frame.push_value::<DUAL_SLOT>(value)?;
            let long_value = i64::from(jvm_instruction.opcode()) - 9;
            let expr = Expression::Const(ConstantValue::Long(long_value));
            Instruction::Definition { value: def, expr }
        }
        FConst0 | FConst1 | FConst2 => {
            let def = required_definition(definition)?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let float_value = f32::from(jvm_instruction.opcode()) - 11.0;
            let expr = Expression::Const(ConstantValue::Float(float_value));
            Instruction::Definition { value: def, expr }
        }
        DConst0 | DConst1 => {
            let def = required_definition(definition)?;
            let value = def.into();
            frame.push_value::<DUAL_SLOT>(value)?;
            let double_value = f64::from(jvm_instruction.opcode()) - 14.0;
            let expr = Expression::Const(ConstantValue::Double(double_value));
            Instruction::Definition { value: def, expr }
        }
        BiPush(value) => {
            let def = required_definition(definition)?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Const(ConstantValue::Integer(i32::from(*value)));
            Instruction::Definition { value: def, expr }
        }
        SiPush(value) => {
            let def = required_definition(definition)?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Const(ConstantValue::Integer(i32::from(*value)));
            Instruction::Definition { value: def, expr }
        }
        Ldc(value) | LdcW(value) => {
            let def = required_definition(definition)?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let expr = Expression::Const(value.clone());
            Instruction::Definition { value: def, expr }
        }
        Ldc2W(value) => {
            let def = required_definition(definition)?;
            frame.push_value::<DUAL_SLOT>(def.into())?;
            let expr = Expression::Const(value.clone());
            Instruction::Definition { value: def, expr }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
