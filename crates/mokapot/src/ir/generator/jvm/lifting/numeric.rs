use crate::{
    ir::{
        expression::{Conversion, MathOperation, NaNTreatment},
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2, Frame},
                instruction::RegisterInstruction,
                lifting::operations::{lift_binary_math, lift_conversion},
                subroutine_expansion::Location,
                symbolic_execution::{Executor, Value},
            },
        },
    },
    jvm::code::Instruction as JVM,
};

#[expect(
    clippy::too_many_lines,
    reason = "the match is an exhaustive opcode-family dispatch"
)]
impl Executor<'_> {
    pub(super) fn try_lift_numeric(
        &mut self,
        jvm_instruction: &JVM,
        location: Location,
        frame: &mut Frame<Value>,
    ) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
        use JVM::{
            D2F, D2I, D2L, DAdd, DCmpG, DCmpL, DDiv, DMul, DNeg, DRem, DSub, F2D, F2I, F2L, FAdd,
            FCmpG, FCmpL, FDiv, FMul, FNeg, FRem, FSub, I2B, I2C, I2D, I2F, I2L, I2S, IAdd, IAnd,
            IDiv, IMul, INeg, IOr, IRem, IShl, IShr, ISub, IUShr, IXor, L2D, L2F, L2I, LAdd, LAnd,
            LCmp, LDiv, LMul, LNeg, LOr, LRem, LShl, LShr, LSub, LUShr, LXor,
        };

        if !matches!(jvm_instruction.opcode(), 96..=131 | 133..=152) {
            return Ok(None);
        }
        let value = self.definition_id_at(location)?;

        let instruction = match jvm_instruction {
            IAdd | FAdd => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::Add)?,
            ISub | FSub => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::Subtract)?,
            IMul | FMul => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::Multiply)?,
            IDiv | FDiv => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::Divide)?,
            IRem | FRem => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::Remainder)?,
            LDiv | DDiv => lift_binary_math::<CATEGORY_2>(frame, value, MathOperation::Divide)?,
            LAdd | DAdd => lift_binary_math::<CATEGORY_2>(frame, value, MathOperation::Add)?,
            LSub | DSub => lift_binary_math::<CATEGORY_2>(frame, value, MathOperation::Subtract)?,
            LMul | DMul => lift_binary_math::<CATEGORY_2>(frame, value, MathOperation::Multiply)?,
            LRem | DRem => lift_binary_math::<CATEGORY_2>(frame, value, MathOperation::Remainder)?,
            INeg | FNeg => {
                let operand = frame.pop_value::<CATEGORY_1>()?;
                frame.push_value::<CATEGORY_1>(value.into())?;
                let expr = MathOperation::Negate(operand).into();
                RegisterInstruction::Definition { value, expr }
            }
            LNeg | DNeg => {
                let operand = frame.pop_value::<CATEGORY_2>()?;
                frame.push_value::<CATEGORY_2>(value.into())?;
                let expr = MathOperation::Negate(operand).into();
                RegisterInstruction::Definition { value, expr }
            }
            IShl => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::ShiftLeft)?,
            IShr => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::ShiftRight)?,
            LShl => {
                let shift_amount = frame.pop_value::<CATEGORY_1>()?;
                let base = frame.pop_value::<CATEGORY_2>()?;
                frame.push_value::<CATEGORY_2>(value.into())?;
                let expr = MathOperation::ShiftLeft(base, shift_amount).into();
                RegisterInstruction::Definition { value, expr }
            }
            LShr => {
                let shift_amount = frame.pop_value::<CATEGORY_1>()?;
                let base = frame.pop_value::<CATEGORY_2>()?;
                frame.push_value::<CATEGORY_2>(value.into())?;
                let expr = MathOperation::ShiftRight(base, shift_amount).into();
                RegisterInstruction::Definition { value, expr }
            }
            LUShr => {
                let shift_amount = frame.pop_value::<CATEGORY_1>()?;
                let base = frame.pop_value::<CATEGORY_2>()?;
                frame.push_value::<CATEGORY_2>(value.into())?;
                let expr = MathOperation::LogicalShiftRight(base, shift_amount).into();
                RegisterInstruction::Definition { value, expr }
            }
            IUShr => {
                lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::LogicalShiftRight)?
            }
            IAnd => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::BitwiseAnd)?,
            IOr => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::BitwiseOr)?,
            IXor => lift_binary_math::<CATEGORY_1>(frame, value, MathOperation::BitwiseXor)?,
            LAnd => lift_binary_math::<CATEGORY_2>(frame, value, MathOperation::BitwiseAnd)?,
            LOr => lift_binary_math::<CATEGORY_2>(frame, value, MathOperation::BitwiseOr)?,
            LXor => lift_binary_math::<CATEGORY_2>(frame, value, MathOperation::BitwiseXor)?,
            I2F => lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, value, Conversion::Int2Float)?,
            I2L => lift_conversion::<CATEGORY_1, CATEGORY_2>(frame, value, Conversion::Int2Long)?,
            I2D => lift_conversion::<CATEGORY_1, CATEGORY_2>(frame, value, Conversion::Int2Double)?,
            L2I => lift_conversion::<CATEGORY_2, CATEGORY_1>(frame, value, Conversion::Long2Int)?,
            L2F => lift_conversion::<CATEGORY_2, CATEGORY_1>(frame, value, Conversion::Long2Float)?,
            L2D => {
                lift_conversion::<CATEGORY_2, CATEGORY_2>(frame, value, Conversion::Long2Double)?
            }
            F2I => lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, value, Conversion::Float2Int)?,
            F2L => lift_conversion::<CATEGORY_1, CATEGORY_2>(frame, value, Conversion::Float2Long)?,
            F2D => {
                lift_conversion::<CATEGORY_1, CATEGORY_2>(frame, value, Conversion::Float2Double)?
            }
            D2I => lift_conversion::<CATEGORY_2, CATEGORY_1>(frame, value, Conversion::Double2Int)?,
            D2L => {
                lift_conversion::<CATEGORY_2, CATEGORY_2>(frame, value, Conversion::Double2Long)?
            }
            D2F => {
                lift_conversion::<CATEGORY_2, CATEGORY_1>(frame, value, Conversion::Double2Float)?
            }
            I2B => lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, value, Conversion::Int2Byte)?,
            I2C => lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, value, Conversion::Int2Char)?,
            I2S => lift_conversion::<CATEGORY_1, CATEGORY_1>(frame, value, Conversion::Int2Short)?,
            LCmp => {
                let rhs = frame.pop_value::<CATEGORY_2>()?;
                let lhs = frame.pop_value::<CATEGORY_2>()?;
                frame.push_value::<CATEGORY_1>(value.into())?;
                let expr = MathOperation::LongComparison(lhs, rhs).into();
                RegisterInstruction::Definition { value, expr }
            }
            FCmpL | FCmpG => {
                let rhs = frame.pop_value::<CATEGORY_1>()?;
                let lhs = frame.pop_value::<CATEGORY_1>()?;
                frame.push_value::<CATEGORY_1>(value.into())?;
                let nan_treatment = match jvm_instruction {
                    FCmpG => NaNTreatment::IsLargest,
                    FCmpL => NaNTreatment::IsSmallest,
                    _ => unreachable!("By outer match arm"),
                };
                let expr = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment).into();
                RegisterInstruction::Definition { value, expr }
            }
            DCmpL | DCmpG => {
                let rhs = frame.pop_value::<CATEGORY_2>()?;
                let lhs = frame.pop_value::<CATEGORY_2>()?;
                frame.push_value::<CATEGORY_1>(value.into())?;
                let nan_treatment = match jvm_instruction {
                    DCmpG => NaNTreatment::IsLargest,
                    DCmpL => NaNTreatment::IsSmallest,
                    _ => unreachable!("By outer match arm"),
                };
                let expr = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment).into();
                RegisterInstruction::Definition { value, expr }
            }
            _ => return Ok(None),
        };
        Ok(Some(instruction))
    }
}
