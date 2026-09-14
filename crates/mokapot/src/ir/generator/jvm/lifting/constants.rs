use crate::{
    ir::{
        expression::Expression,
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2, Frame},
                instruction::RegisterInstruction,
                symbolic_execution::Value,
            },
        },
    },
    jvm::{ConstantValue, code::Instruction as JVM},
};

pub(super) const fn produces_value(instruction: &JVM) -> bool {
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

pub(super) fn try_lift(
    jvm_instruction: &JVM,
    value: SsaValueId,
    frame: &mut Frame<Value>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    use JVM::{
        AConstNull, BiPush, DConst0, DConst1, FConst0, FConst1, FConst2, IConst0, IConst1, IConst2,
        IConst3, IConst4, IConst5, IConstM1, LConst0, LConst1, Ldc, Ldc2W, LdcW, SiPush,
    };

    let instruction = match jvm_instruction {
        AConstNull => {
            frame.push_value::<CATEGORY_1>(value.into())?;
            let expr = Expression::Const(ConstantValue::Null);
            RegisterInstruction::Definition { value, expr }
        }
        IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
            frame.push_value::<CATEGORY_1>(value.into())?;
            let int_value = i32::from(jvm_instruction.opcode()) - 3;
            let expr = Expression::Const(ConstantValue::Integer(int_value));
            RegisterInstruction::Definition { value, expr }
        }
        LConst0 | LConst1 => {
            frame.push_value::<CATEGORY_2>(value.into())?;
            let long_value = i64::from(jvm_instruction.opcode()) - 9;
            let expr = Expression::Const(ConstantValue::Long(long_value));
            RegisterInstruction::Definition { value, expr }
        }
        FConst0 | FConst1 | FConst2 => {
            frame.push_value::<CATEGORY_1>(value.into())?;
            let float_value = f32::from(jvm_instruction.opcode()) - 11.0;
            let expr = Expression::Const(ConstantValue::Float(float_value));
            RegisterInstruction::Definition { value, expr }
        }
        DConst0 | DConst1 => {
            frame.push_value::<CATEGORY_2>(value.into())?;
            let double_value = f64::from(jvm_instruction.opcode()) - 14.0;
            let expr = Expression::Const(ConstantValue::Double(double_value));
            RegisterInstruction::Definition { value, expr }
        }
        BiPush(jvm_value) => {
            frame.push_value::<CATEGORY_1>(value.into())?;
            let expr = Expression::Const(ConstantValue::Integer(i32::from(*jvm_value)));
            RegisterInstruction::Definition { value, expr }
        }
        SiPush(jvm_value) => {
            frame.push_value::<CATEGORY_1>(value.into())?;
            let expr = Expression::Const(ConstantValue::Integer(i32::from(*jvm_value)));
            RegisterInstruction::Definition { value, expr }
        }
        Ldc(jvm_value) | LdcW(jvm_value) => {
            frame.push_value::<CATEGORY_1>(value.into())?;
            let expr = Expression::Const(jvm_value.clone());
            RegisterInstruction::Definition { value, expr }
        }
        Ldc2W(jvm_value) => {
            frame.push_value::<CATEGORY_2>(value.into())?;
            let expr = Expression::Const(jvm_value.clone());
            RegisterInstruction::Definition { value, expr }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
