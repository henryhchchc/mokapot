#[allow(
    clippy::wildcard_imports,
    reason = "opcode-family lifters share the private lifting vocabulary"
)]
use super::*;

#[expect(
    clippy::too_many_lines,
    reason = "the match is an exhaustive opcode-family dispatch"
)]
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
        IAdd | FAdd => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::Add)?,
        ISub | FSub => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::Subtract)?,
        IMul | FMul => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::Multiply)?,
        IDiv | FDiv => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::Divide)?,
        IRem | FRem => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::Remainder)?,
        LDiv | DDiv => binary_op_math::<DUAL_SLOT, _>(frame, def, MathOperation::Divide)?,
        LAdd | DAdd => binary_op_math::<DUAL_SLOT, _>(frame, def, MathOperation::Add)?,
        LSub | DSub => binary_op_math::<DUAL_SLOT, _>(frame, def, MathOperation::Subtract)?,
        LMul | DMul => binary_op_math::<DUAL_SLOT, _>(frame, def, MathOperation::Multiply)?,
        LRem | DRem => binary_op_math::<DUAL_SLOT, _>(frame, def, MathOperation::Remainder)?,
        INeg | FNeg => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let math_op = MathOperation::Negate(value);
            IR::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        LNeg | DNeg => {
            let operand = frame.pop_value::<DUAL_SLOT>()?;
            let value = def.into();
            frame.push_value::<DUAL_SLOT>(value)?;
            let math_op = MathOperation::Negate(operand);
            IR::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        IShl => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::ShiftLeft)?,
        IShr => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::ShiftRight)?,
        LShl => {
            let shift_amount = frame.pop_value::<SINGLE_SLOT>()?;
            let base = frame.pop_value::<DUAL_SLOT>()?;
            let value = def.into();
            frame.push_value::<DUAL_SLOT>(value)?;
            let math_op = MathOperation::ShiftLeft(base, shift_amount);
            IR::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        LShr => {
            let shift_amount = frame.pop_value::<SINGLE_SLOT>()?;
            let base = frame.pop_value::<DUAL_SLOT>()?;
            frame.push_value::<DUAL_SLOT>(def.into())?;
            let math_op = MathOperation::ShiftRight(base, shift_amount);
            IR::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        LUShr => {
            let shift_amount = frame.pop_value::<SINGLE_SLOT>()?;
            let base = frame.pop_value::<DUAL_SLOT>()?;
            frame.push_value::<DUAL_SLOT>(def.into())?;
            let math_op = MathOperation::LogicalShiftRight(base, shift_amount);
            IR::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        IUShr => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::LogicalShiftRight)?,
        IAnd => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::BitwiseAnd)?,
        IOr => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::BitwiseOr)?,
        IXor => binary_op_math::<SINGLE_SLOT, _>(frame, def, MathOperation::BitwiseXor)?,
        LAnd => binary_op_math::<DUAL_SLOT, _>(frame, def, MathOperation::BitwiseAnd)?,
        LOr => binary_op_math::<DUAL_SLOT, _>(frame, def, MathOperation::BitwiseOr)?,
        LXor => binary_op_math::<DUAL_SLOT, _>(frame, def, MathOperation::BitwiseXor)?,
        IInc(idx, constant) => {
            let idx = (*idx).into();
            let base = frame.get_local::<SINGLE_SLOT>(idx)?;
            frame.set_local::<SINGLE_SLOT>(idx, def.into())?;
            let math_op = MathOperation::Increment(base, *constant);
            IR::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        Wide(WideInstruction::IInc(idx, constant)) => {
            let base = frame.get_local::<SINGLE_SLOT>(*idx)?;
            frame.set_local::<SINGLE_SLOT>(*idx, def.into())?;
            let math_op = MathOperation::Increment(base, *constant);
            IR::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        I2F => conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Int2Float)?,
        I2L => conversion_op::<SINGLE_SLOT, DUAL_SLOT, _>(frame, def, Conversion::Int2Long)?,
        I2D => conversion_op::<SINGLE_SLOT, DUAL_SLOT, _>(frame, def, Conversion::Int2Double)?,
        L2I => conversion_op::<DUAL_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Long2Int)?,
        L2F => conversion_op::<DUAL_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Long2Float)?,
        L2D => conversion_op::<DUAL_SLOT, DUAL_SLOT, _>(frame, def, Conversion::Long2Double)?,
        F2I => conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Float2Int)?,
        F2L => conversion_op::<SINGLE_SLOT, DUAL_SLOT, _>(frame, def, Conversion::Float2Long)?,
        F2D => conversion_op::<SINGLE_SLOT, DUAL_SLOT, _>(frame, def, Conversion::Float2Double)?,
        D2I => conversion_op::<DUAL_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Double2Int)?,
        D2L => conversion_op::<DUAL_SLOT, DUAL_SLOT, _>(frame, def, Conversion::Double2Long)?,
        D2F => conversion_op::<DUAL_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Double2Float)?,
        I2B => conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Int2Byte)?,
        I2C => conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Int2Char)?,
        I2S => conversion_op::<SINGLE_SLOT, SINGLE_SLOT, _>(frame, def, Conversion::Int2Short)?,
        LCmp => {
            let rhs = frame.pop_value::<DUAL_SLOT>()?;
            let lhs = frame.pop_value::<DUAL_SLOT>()?;
            frame.push_value::<SINGLE_SLOT>(def.into())?;
            let math_op = MathOperation::LongComparison(lhs, rhs);
            IR::Definition {
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
            IR::Definition {
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
            IR::Definition {
                value: def,
                expr: Expression::Math(math_op),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
