use crate::{
    ir::{
        expression::{Conversion, Expression, MathOperation, NaNTreatment},
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{DUAL_SLOT, JvmStackFrame, SINGLE_SLOT},
                instruction::RegisterInstruction,
                lifting::{
                    operations::{lift_binary_math, lift_conversion},
                    require_definition_id,
                },
                symbolic_execution::SymbolicValue,
            },
        },
    },
    jvm::code::Instruction as JVM,
};

pub(super) const fn produces_value(instruction: &JVM) -> bool {
    matches!(instruction.opcode(), 96..=131 | 133..=152)
}

#[expect(
    clippy::too_many_lines,
    reason = "the match is an exhaustive opcode-family dispatch"
)]
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

    if !produces_value(jvm_instruction) {
        return Ok(None);
    }
    let def = require_definition_id(definition)?;

    let instruction = match jvm_instruction {
        IAdd | FAdd => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::Add)?,
        ISub | FSub => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::Subtract)?,
        IMul | FMul => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::Multiply)?,
        IDiv | FDiv => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::Divide)?,
        IRem | FRem => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::Remainder)?,
        LDiv | DDiv => lift_binary_math::<DUAL_SLOT>(frame, def, MathOperation::Divide)?,
        LAdd | DAdd => lift_binary_math::<DUAL_SLOT>(frame, def, MathOperation::Add)?,
        LSub | DSub => lift_binary_math::<DUAL_SLOT>(frame, def, MathOperation::Subtract)?,
        LMul | DMul => lift_binary_math::<DUAL_SLOT>(frame, def, MathOperation::Multiply)?,
        LRem | DRem => lift_binary_math::<DUAL_SLOT>(frame, def, MathOperation::Remainder)?,
        INeg | FNeg => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let math_op = MathOperation::Negate(value);
            RegisterInstruction::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        LNeg | DNeg => {
            let operand = frame.pop_value::<DUAL_SLOT>()?;
            let value = def.into();
            frame.push_value::<DUAL_SLOT>(value)?;
            let math_op = MathOperation::Negate(operand);
            RegisterInstruction::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        IShl => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::ShiftLeft)?,
        IShr => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::ShiftRight)?,
        LShl => {
            let shift_amount = frame.pop_value::<SINGLE_SLOT>()?;
            let base = frame.pop_value::<DUAL_SLOT>()?;
            let value = def.into();
            frame.push_value::<DUAL_SLOT>(value)?;
            let math_op = MathOperation::ShiftLeft(base, shift_amount);
            RegisterInstruction::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        LShr => {
            let shift_amount = frame.pop_value::<SINGLE_SLOT>()?;
            let base = frame.pop_value::<DUAL_SLOT>()?;
            frame.push_value::<DUAL_SLOT>(def.into())?;
            let math_op = MathOperation::ShiftRight(base, shift_amount);
            RegisterInstruction::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        LUShr => {
            let shift_amount = frame.pop_value::<SINGLE_SLOT>()?;
            let base = frame.pop_value::<DUAL_SLOT>()?;
            frame.push_value::<DUAL_SLOT>(def.into())?;
            let math_op = MathOperation::LogicalShiftRight(base, shift_amount);
            RegisterInstruction::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        IUShr => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::LogicalShiftRight)?,
        IAnd => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::BitwiseAnd)?,
        IOr => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::BitwiseOr)?,
        IXor => lift_binary_math::<SINGLE_SLOT>(frame, def, MathOperation::BitwiseXor)?,
        LAnd => lift_binary_math::<DUAL_SLOT>(frame, def, MathOperation::BitwiseAnd)?,
        LOr => lift_binary_math::<DUAL_SLOT>(frame, def, MathOperation::BitwiseOr)?,
        LXor => lift_binary_math::<DUAL_SLOT>(frame, def, MathOperation::BitwiseXor)?,
        I2F => lift_conversion::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Int2Float)?,
        I2L => lift_conversion::<SINGLE_SLOT, DUAL_SLOT>(frame, def, Conversion::Int2Long)?,
        I2D => lift_conversion::<SINGLE_SLOT, DUAL_SLOT>(frame, def, Conversion::Int2Double)?,
        L2I => lift_conversion::<DUAL_SLOT, SINGLE_SLOT>(frame, def, Conversion::Long2Int)?,
        L2F => lift_conversion::<DUAL_SLOT, SINGLE_SLOT>(frame, def, Conversion::Long2Float)?,
        L2D => lift_conversion::<DUAL_SLOT, DUAL_SLOT>(frame, def, Conversion::Long2Double)?,
        F2I => lift_conversion::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Float2Int)?,
        F2L => lift_conversion::<SINGLE_SLOT, DUAL_SLOT>(frame, def, Conversion::Float2Long)?,
        F2D => lift_conversion::<SINGLE_SLOT, DUAL_SLOT>(frame, def, Conversion::Float2Double)?,
        D2I => lift_conversion::<DUAL_SLOT, SINGLE_SLOT>(frame, def, Conversion::Double2Int)?,
        D2L => lift_conversion::<DUAL_SLOT, DUAL_SLOT>(frame, def, Conversion::Double2Long)?,
        D2F => lift_conversion::<DUAL_SLOT, SINGLE_SLOT>(frame, def, Conversion::Double2Float)?,
        I2B => lift_conversion::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Int2Byte)?,
        I2C => lift_conversion::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Int2Char)?,
        I2S => lift_conversion::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Int2Short)?,
        LCmp => {
            let rhs = frame.pop_value::<DUAL_SLOT>()?;
            let lhs = frame.pop_value::<DUAL_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let math_op = MathOperation::LongComparison(lhs, rhs);
            RegisterInstruction::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        FCmpL | FCmpG => {
            let rhs = frame.pop_value::<SINGLE_SLOT>()?;
            let lhs = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let nan_treatment = match jvm_instruction {
                FCmpG => NaNTreatment::IsLargest,
                FCmpL => NaNTreatment::IsSmallest,
                _ => unreachable!("By outer match arm"),
            };
            let math_op = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment);
            RegisterInstruction::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        DCmpL | DCmpG => {
            let rhs = frame.pop_value::<DUAL_SLOT>()?;
            let lhs = frame.pop_value::<DUAL_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let nan_treatment = match jvm_instruction {
                DCmpG => NaNTreatment::IsLargest,
                DCmpL => NaNTreatment::IsSmallest,
                _ => unreachable!("By outer match arm"),
            };
            let math_op = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment);
            RegisterInstruction::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
