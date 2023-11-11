use std::iter::once;

use crate::{
    analysis::moka_ir::{
        moka_instruction::{Identifier, MokaInstruction as IR},
        ArrayOperation, Expression, FieldAccess, MathOperation, MonitorOperation, NanTreatment,
    },
    elements::{
        instruction::{Instruction, ProgramCounter, TypeReference},
        ConstantValue, ReturnType,
    },
    types::{FieldType, PrimitiveType},
};

use super::{
    Condition, ConversionOperation, MokaIRGenerationError, MokaIRGenerator, StackFrame, ValueRef,
};

const LONG_TYPE: FieldType = FieldType::Base(PrimitiveType::Long);
const DOUBLE_TYPE: FieldType = FieldType::Base(PrimitiveType::Double);
const LONG_RET_TYPE: ReturnType = ReturnType::Some(LONG_TYPE);
const DOUBLE_RET_TYPE: ReturnType = ReturnType::Some(DOUBLE_TYPE);

impl MokaIRGenerator {
    pub(super) fn run_instruction(
        &mut self,
        insn: &Instruction,
        pc: ProgramCounter,
        frame: &mut StackFrame,
    ) -> Result<(), MokaIRGenerationError> {
        use Instruction::*;
        let def_id = Identifier::Val(pc.into());
        let ir_instruction = match insn {
            Nop => IR::Nop,
            AConstNull => {
                let constant = ConstantValue::Null;
                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(constant),
                }
            }
            IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
                let constant = ConstantValue::Integer((insn.opcode() as i32) - 3);
                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(constant),
                }
            }
            LConst0 | LConst1 => {
                let constant = ConstantValue::Long((insn.opcode() as i64) - 9);
                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(constant),
                }
            }
            FConst0 | FConst1 | FConst2 => {
                frame.push_value(def_id.into())?;
                let constant = ConstantValue::Float((insn.opcode() as f32) - 11.0);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(constant),
                }
            }
            DConst0 | DConst1 => {
                frame.push_value(def_id.into())?;
                let constant = ConstantValue::Double((insn.opcode() as f64) - 14.0);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(constant),
                }
            }
            BiPush(value) => {
                frame.push_value(def_id.into())?;
                let constant = ConstantValue::Integer(*value as i32);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(constant),
                }
            }
            SiPush(value) => {
                let value = *value as i32;
                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(ConstantValue::Integer(value)),
                }
            }
            Ldc(value) | LdcW(value) => {
                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(value.clone()),
                }
            }
            Ldc2W(value) => {
                frame.push_value(def_id.into())?;
                frame.push_padding()?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Const(value.clone()),
                }
            }
            ILoad(idx) | FLoad(idx) | ALoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value)?;
                IR::Nop
            }
            LLoad(idx) | DLoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value)?;
                frame.push_padding()?;
                IR::Nop
            }
            ILoad0 | FLoad0 | ALoad0 => {
                let value = frame.get_local(0usize)?;
                frame.push_value(value)?;
                IR::Nop
            }
            ILoad1 | FLoad1 | ALoad1 => {
                let value = frame.get_local(1usize)?;
                frame.push_value(value)?;
                IR::Nop
            }
            ILoad2 | FLoad2 | ALoad2 => {
                let value = frame.get_local(2usize)?;
                frame.push_value(value)?;
                IR::Nop
            }
            ILoad3 | FLoad3 | ALoad3 => {
                let value = frame.get_local(3usize)?;
                frame.push_value(value)?;
                IR::Nop
            }
            LLoad0 | DLoad0 => {
                let value = frame.get_local(0usize)?;
                frame.push_value(value)?;
                frame.push_padding()?;
                IR::Nop
            }
            LLoad1 | DLoad1 => {
                let value = frame.get_local(1usize)?;
                frame.push_value(value)?;
                frame.push_padding()?;
                IR::Nop
            }
            LLoad2 | DLoad2 => {
                let value = frame.get_local(2usize)?;
                frame.push_value(value)?;
                frame.push_padding()?;
                IR::Nop
            }
            LLoad3 | DLoad3 => {
                let value = frame.get_local(3usize)?;
                frame.push_value(value)?;
                frame.push_padding()?;
                IR::Nop
            }
            IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Read { array_ref, index };

                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Array(array_op),
                }
            }
            LALoad | DALoad => {
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Read { array_ref, index };

                frame.push_value(def_id.into())?;
                frame.push_padding()?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Array(array_op),
                }
            }
            IStore(idx) | FStore(idx) | AStore(idx) => {
                let value = frame.pop_value()?;
                frame.set_local(*idx, value)?;
                IR::Nop
            }
            LStore(idx) | DStore(idx) => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;
                frame.set_local(*idx, value)?;
                frame.set_local_padding(*idx + 1)?;
                IR::Nop
            }
            IStore0 | FStore0 | AStore0 => {
                let value = frame.pop_value()?;
                frame.set_local(0usize, value)?;
                IR::Nop
            }
            IStore1 | FStore1 | AStore1 => {
                let value = frame.pop_value()?;
                frame.set_local(1usize, value)?;
                IR::Nop
            }
            IStore2 | FStore2 | AStore2 => {
                let value = frame.pop_value()?;
                frame.set_local(2usize, value)?;
                IR::Nop
            }
            IStore3 | FStore3 | AStore3 => {
                let value = frame.pop_value()?;
                frame.set_local(3usize, value)?;
                IR::Nop
            }
            LStore0 | DStore0 => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;
                frame.set_local(0usize, value)?;
                frame.set_local_padding(1usize)?;
                IR::Nop
            }
            LStore1 | DStore1 => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;
                frame.set_local(1usize, value)?;
                frame.set_local_padding(2usize)?;
                IR::Nop
            }
            LStore2 | DStore2 => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;
                frame.set_local(2usize, value)?;
                frame.set_local_padding(2usize)?;
                IR::Nop
            }
            LStore3 | DStore3 => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;
                frame.set_local(3usize, value)?;
                frame.set_local_padding(4usize)?;
                IR::Nop
            }
            IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
                let value = frame.pop_value()?;
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Write {
                    array_ref,
                    index,
                    value,
                };

                IR::SideEffect {
                    rhs: Expression::Array(array_op),
                }
            }
            LAStore | DAStore => {
                let _value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Write {
                    array_ref,
                    index,
                    value,
                };
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Array(array_op),
                }
            }
            Pop => {
                let _discarded = frame.pop_raw()?;
                IR::Nop
            }
            Pop2 => {
                let _discarded1 = frame.pop_raw()?;
                let _discarded2 = frame.pop_raw()?;
                IR::Nop
            }
            Dup => {
                let value = frame.pop_raw()?;
                frame.push_raw(value.clone())?;
                frame.push_raw(value)?;
                IR::Nop
            }
            DupX1 => {
                let value1 = frame.pop_raw()?;
                let value2 = frame.pop_raw()?;
                frame.push_raw(value1.clone())?;
                frame.push_raw(value2)?;
                frame.push_raw(value1)?;
                IR::Nop
            }
            DupX2 => {
                let value1 = frame.pop_raw()?;
                let value2 = frame.pop_raw()?;
                let value3 = frame.pop_raw()?;
                frame.push_raw(value1.clone())?;
                frame.push_raw(value3)?;
                frame.push_raw(value2)?;
                frame.push_raw(value1)?;
                IR::Nop
            }
            Dup2 => {
                let value1 = frame.pop_raw()?;
                let value2 = frame.pop_raw()?;
                frame.push_raw(value2.clone())?;
                frame.push_raw(value1.clone())?;
                frame.push_raw(value2)?;
                frame.push_raw(value1)?;
                IR::Nop
            }
            Dup2X1 => {
                let value1 = frame.pop_raw()?;
                let value2 = frame.pop_raw()?;
                let value3 = frame.pop_raw()?;
                frame.push_raw(value2.clone())?;
                frame.push_raw(value1.clone())?;
                frame.push_raw(value3)?;
                frame.push_raw(value2)?;
                frame.push_raw(value1)?;
                IR::Nop
            }
            Dup2X2 => {
                let value1 = frame.pop_raw()?;
                let value2 = frame.pop_raw()?;
                let value3 = frame.pop_raw()?;
                let value4 = frame.pop_raw()?;
                frame.push_raw(value2.clone())?;
                frame.push_raw(value1.clone())?;
                frame.push_raw(value4)?;
                frame.push_raw(value3)?;
                frame.push_raw(value2)?;
                frame.push_raw(value1)?;
                IR::Nop
            }
            Swap => {
                let value1 = frame.pop_raw()?;
                let value2 = frame.pop_raw()?;
                frame.push_raw(value1)?;
                frame.push_raw(value2)?;
                IR::Nop
            }
            IAdd | FAdd => self.binary_op_math(frame, def_id, MathOperation::Add)?,
            LAdd | DAdd => self.binary_wide_math(frame, def_id, MathOperation::Add)?,
            ISub | FSub => self.binary_op_math(frame, def_id, MathOperation::Subtract)?,
            LSub | DSub => self.binary_wide_math(frame, def_id, MathOperation::Subtract)?,
            IMul | FMul => self.binary_op_math(frame, def_id, MathOperation::Multiply)?,
            LMul | DMul => self.binary_wide_math(frame, def_id, MathOperation::Multiply)?,
            IDiv | FDiv => self.binary_op_math(frame, def_id, MathOperation::Divide)?,
            LDiv | DDiv => self.binary_wide_math(frame, def_id, MathOperation::Divide)?,
            IRem | FRem => self.binary_op_math(frame, def_id, MathOperation::Remainder)?,
            LRem | DRem => self.binary_wide_math(frame, def_id, MathOperation::Remainder)?,
            INeg | FNeg => {
                let value = frame.pop_value()?;
                frame.push_value(def_id.into())?;
                let math_op = MathOperation::Negate(value);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Math(math_op),
                }
            }
            LNeg | DNeg => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;
                frame.push_value(def_id.into())?;
                frame.push_padding()?;
                let math_op = MathOperation::Negate(value);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Math(math_op),
                }
            }
            IShl => self.binary_op_math(frame, def_id, MathOperation::ShiftLeft)?,
            LShl => self.binary_wide_math(frame, def_id, MathOperation::ShiftLeft)?,
            IShr => self.binary_op_math(frame, def_id, MathOperation::ShiftRight)?,
            LShr => self.binary_wide_math(frame, def_id, MathOperation::ShiftRight)?,
            IUShr => self.binary_op_math(frame, def_id, MathOperation::LogicalShiftRight)?,
            LUShr => self.binary_wide_math(frame, def_id, MathOperation::LogicalShiftRight)?,
            IAnd => self.binary_op_math(frame, def_id, MathOperation::BitwiseAnd)?,
            LAnd => self.binary_wide_math(frame, def_id, MathOperation::BitwiseAnd)?,
            IOr => self.binary_op_math(frame, def_id, MathOperation::BitwiseOr)?,
            LOr => self.binary_wide_math(frame, def_id, MathOperation::BitwiseOr)?,
            IXor => self.binary_op_math(frame, def_id, MathOperation::BitwiseXor)?,
            LXor => self.binary_wide_math(frame, def_id, MathOperation::BitwiseXor)?,
            IInc(idx, _) => {
                let base = frame.get_local(*idx)?;
                frame.set_local(*idx, def_id.into())?;
                let math_op = MathOperation::Increment(base);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Math(math_op),
                }
            }
            WideIInc(idx, _) => {
                let base = frame.get_local(*idx)?;
                frame.set_local(*idx, def_id.into())?;
                let math_op = MathOperation::Increment(base);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Math(math_op),
                }
            }
            I2F => self.conversion_op::<_, false, false>(
                frame,
                def_id,
                ConversionOperation::Int2Float,
            )?,
            I2L => {
                self.conversion_op::<_, false, true>(frame, def_id, ConversionOperation::Int2Long)?
            }
            I2D => self.conversion_op::<_, false, true>(
                frame,
                def_id,
                ConversionOperation::Int2Double,
            )?,
            L2I => {
                self.conversion_op::<_, true, false>(frame, def_id, ConversionOperation::Long2Int)?
            }
            L2F => self.conversion_op::<_, true, false>(
                frame,
                def_id,
                ConversionOperation::Long2Float,
            )?,
            L2D => self.conversion_op::<_, true, true>(
                frame,
                def_id,
                ConversionOperation::Long2Double,
            )?,
            F2I => self.conversion_op::<_, false, false>(
                frame,
                def_id,
                ConversionOperation::Float2Int,
            )?,
            F2L => self.conversion_op::<_, false, true>(
                frame,
                def_id,
                ConversionOperation::Float2Long,
            )?,
            F2D => self.conversion_op::<_, false, true>(
                frame,
                def_id,
                ConversionOperation::Float2Double,
            )?,
            D2I => self.conversion_op::<_, true, false>(
                frame,
                def_id,
                ConversionOperation::Double2Int,
            )?,
            D2L => self.conversion_op::<_, true, true>(
                frame,
                def_id,
                ConversionOperation::Double2Long,
            )?,
            D2F => self.conversion_op::<_, true, false>(
                frame,
                def_id,
                ConversionOperation::Double2Float,
            )?,
            I2B => {
                self.conversion_op::<_, false, false>(frame, def_id, ConversionOperation::Int2Byte)?
            }
            I2C => {
                self.conversion_op::<_, false, false>(frame, def_id, ConversionOperation::Int2Char)?
            }
            I2S => self.conversion_op::<_, false, false>(
                frame,
                def_id,
                ConversionOperation::Int2Short,
            )?,
            LCmp => {
                let lhs = {
                    frame.pop_padding()?;
                    frame.pop_value()?
                };
                let rhs = {
                    frame.pop_padding()?;
                    frame.pop_value()?
                };
                frame.push_value(def_id.into())?;
                let math_op = MathOperation::LongComparison(lhs, rhs);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Math(math_op),
                }
            }
            FCmpL | FCmpG | DCmpL | DCmpG => {
                let lhs = {
                    frame.pop_padding()?;
                    frame.pop_value()?
                };
                let rhs = {
                    frame.pop_padding()?;
                    frame.pop_value()?
                };
                frame.push_value(def_id.into())?;
                let nan_treatment = match insn {
                    FCmpG | DCmpG => NanTreatment::IsLargest,
                    FCmpL | DCmpL => NanTreatment::IsSmallest,
                    _ => unreachable!(),
                };
                let math_op = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment);
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Math(math_op),
                }
            }
            IfEq(target) => self.unitary_conditional_jump(frame, *target, Condition::Zero)?,
            IfNe(target) => self.unitary_conditional_jump(frame, *target, Condition::NonZero)?,
            IfLt(target) => self.unitary_conditional_jump(frame, *target, Condition::Negative)?,
            IfGe(target) => {
                self.unitary_conditional_jump(frame, *target, Condition::NonNegative)?
            }
            IfGt(target) => self.unitary_conditional_jump(frame, *target, Condition::Positive)?,
            IfLe(target) => {
                self.unitary_conditional_jump(frame, *target, Condition::NonPositive)?
            }
            IfNull(target) => self.unitary_conditional_jump(frame, *target, Condition::IsNull)?,
            IfNonNull(target) => {
                self.unitary_conditional_jump(frame, *target, Condition::IsNotNull)?
            }
            IfICmpEq(target) | IfACmpEq(target) => {
                self.binary_conditional_jump(frame, *target, Condition::Equal)?
            }
            IfICmpNe(target) | IfACmpNe(target) => {
                self.binary_conditional_jump(frame, *target, Condition::NotEqual)?
            }
            IfICmpGe(target) => {
                self.binary_conditional_jump(frame, *target, Condition::GreaterThanOrEqual)?
            }
            IfICmpLt(target) => {
                self.binary_conditional_jump(frame, *target, Condition::LessThan)?
            }
            IfICmpGt(target) => {
                self.binary_conditional_jump(frame, *target, Condition::GreaterThan)?
            }
            IfICmpLe(target) => {
                self.binary_conditional_jump(frame, *target, Condition::LessThanOrEqual)?
            }
            Goto(target) | GotoW(target) => IR::Jump {
                condition: None,
                target: *target,
            },
            Jsr(_) | JsrW(_) => {
                let value = Expression::ReturnAddress(pc);
                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: value,
                }
            }
            Ret(idx) => {
                let return_address = frame.get_local(*idx)?;
                IR::SubRoutineRet {
                    target: return_address,
                }
            }
            WideRet(idx) => {
                let return_address = frame.get_local(*idx)?;
                IR::SubRoutineRet {
                    target: return_address,
                }
            }
            TableSwitch { .. } | LookupSwitch { .. } => {
                let condition = frame.pop_value()?;
                IR::Switch {
                    match_value: condition,
                    instruction: insn.clone(),
                }
            }
            IReturn | FReturn | AReturn => {
                let value = frame.pop_value()?;
                IR::Return { value: Some(value) }
            }
            LReturn | DReturn => {
                let value = {
                    frame.pop_padding()?;
                    frame.pop_value()?
                };
                IR::Return { value: Some(value) }
            }
            Return => IR::Return { value: None },
            GetStatic(field) => {
                frame.push_value(def_id.into())?;
                if let LONG_TYPE | DOUBLE_TYPE = field.field_type {
                    frame.push_padding()?;
                }
                let field_op = FieldAccess::ReadStatic {
                    field: field.clone(),
                };
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Field(field_op),
                }
            }
            GetField(field) => {
                let object_ref = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                if let LONG_TYPE | DOUBLE_TYPE = field.field_type {
                    frame.push_padding()?;
                }
                let field_op = FieldAccess::ReadInstance {
                    object_ref,
                    field: field.clone(),
                };
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Field(field_op),
                }
            }
            PutStatic(field) => {
                if let LONG_TYPE | DOUBLE_TYPE = field.field_type {
                    frame.pop_value()?;
                }
                let value = frame.pop_value()?;
                let field_op = FieldAccess::WriteStatic {
                    field: field.clone(),
                    value,
                };
                IR::SideEffect {
                    rhs: Expression::Field(field_op),
                }
            }
            PutField(field) => {
                let value = match field.field_type {
                    LONG_TYPE | DOUBLE_TYPE => {
                        frame.pop_padding()?;
                        frame.pop_value()?
                    }
                    _ => frame.pop_value()?,
                };
                let object_ref = frame.pop_value()?;
                let field_op = FieldAccess::WriteInstance {
                    object_ref,
                    field: field.clone(),
                    value,
                };
                IR::SideEffect {
                    rhs: Expression::Field(field_op),
                }
            }
            InvokeVirtual(method_ref) | InvokeSpecial(method_ref) => {
                let arguments: Vec<_> = method_ref
                    .descriptor()
                    .parameters_types
                    .iter()
                    .map(|_| frame.pop_value())
                    .collect::<Result<_, _>>()?;
                let object_ref = frame.pop_value()?;
                let arguments = once(object_ref)
                    .chain(arguments.into_iter().rev())
                    .collect();

                let rhs = Expression::Insn {
                    instruction: insn.clone(),
                    arguments,
                };
                match method_ref.descriptor().return_type {
                    ReturnType::Void => IR::SideEffect { rhs },
                    LONG_RET_TYPE | DOUBLE_RET_TYPE => {
                        frame.push_value(def_id.into())?;
                        frame.push_padding()?;
                        IR::Assignment { lhs: def_id, rhs }
                    }
                    _ => {
                        frame.push_value(def_id.into())?;
                        IR::Assignment { lhs: def_id, rhs }
                    }
                }
            }
            InvokeInterface(i_method_ref, _) => {
                let arguments: Vec<_> = i_method_ref
                    .descriptor
                    .parameters_types
                    .iter()
                    .map(|_| frame.pop_value())
                    .collect::<Result<_, _>>()?;
                let object_ref = frame.pop_value()?;
                let arguments = once(object_ref)
                    .chain(arguments.into_iter().rev())
                    .collect();

                let rhs = Expression::Insn {
                    instruction: insn.clone(),
                    arguments,
                };
                match i_method_ref.descriptor.return_type {
                    ReturnType::Void => IR::SideEffect { rhs },
                    LONG_RET_TYPE | DOUBLE_RET_TYPE => {
                        frame.push_value(def_id.into())?;
                        frame.push_padding()?;
                        IR::Assignment { lhs: def_id, rhs }
                    }
                    _ => {
                        frame.push_value(def_id.into())?;
                        IR::Assignment { lhs: def_id, rhs }
                    }
                }
            }
            InvokeStatic(method_ref) => {
                let mut arguments: Vec<_> = method_ref
                    .descriptor()
                    .parameters_types
                    .iter()
                    .map(|_| frame.pop_value())
                    .collect::<Result<_, _>>()?;

                arguments.reverse();

                let rhs = Expression::Insn {
                    instruction: insn.clone(),
                    arguments,
                };
                match method_ref.descriptor().return_type {
                    ReturnType::Void => IR::SideEffect { rhs },
                    LONG_RET_TYPE | DOUBLE_RET_TYPE => {
                        frame.push_value(def_id.into())?;
                        frame.push_padding()?;
                        IR::Assignment { lhs: def_id, rhs }
                    }
                    _ => {
                        frame.push_value(def_id.into())?;
                        IR::Assignment { lhs: def_id, rhs }
                    }
                }
            }
            InvokeDynamic { descriptor, .. } => {
                let arguments: Vec<_> = descriptor
                    .parameters_types
                    .iter()
                    .map(|_| frame.pop_value())
                    .rev()
                    .collect::<Result<_, _>>()?;

                let rhs = Expression::Insn {
                    instruction: insn.clone(),
                    arguments,
                };
                match descriptor.return_type {
                    ReturnType::Void => IR::SideEffect { rhs },
                    LONG_RET_TYPE | DOUBLE_RET_TYPE => {
                        frame.push_value(def_id.into())?;
                        frame.push_padding()?;
                        IR::Assignment { lhs: def_id, rhs }
                    }
                    _ => {
                        frame.push_value(def_id.into())?;
                        IR::Assignment { lhs: def_id, rhs }
                    }
                }
            }
            New(_) => {
                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![],
                    },
                }
            }
            ANewArray(class_ref) => {
                let count = frame.pop_value()?;
                frame.push_value(def_id.into())?;
                let array_op = ArrayOperation::New {
                    element_type: FieldType::Object(class_ref.clone()),
                    length: count,
                };
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Array(array_op),
                }
            }
            NewArray(prim_type) => {
                let count = frame.pop_value()?;
                frame.push_value(def_id.into())?;
                let array_op = ArrayOperation::New {
                    element_type: FieldType::Base(*prim_type),
                    length: count,
                };
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Array(array_op),
                }
            }
            MultiANewArray(TypeReference(element_type), dimension) => {
                let counts: Vec<_> = (0..*dimension)
                    .map(|_| frame.pop_value())
                    .collect::<Result<_, _>>()?;
                frame.push_value(def_id.into())?;
                let array_op = ArrayOperation::NewMultiDim {
                    element_type: element_type.clone(),
                    dimensions: counts,
                };
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Array(array_op),
                }
            }
            ArrayLength => {
                let array_ref = frame.pop_value()?;
                frame.push_value(def_id.into())?;
                let array_op = ArrayOperation::Length { array_ref };
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Array(array_op),
                }
            }
            AThrow => {
                let exception_ref = frame.pop_value()?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Throw(exception_ref),
                }
            }
            CheckCast(TypeReference(target_type)) => {
                self.conversion_op::<_, false, false>(frame, def_id, |value| {
                    ConversionOperation::CheckCast {
                        value,
                        target_type: target_type.clone(),
                    }
                })?
            }
            InstanceOf(TypeReference(target_type)) => {
                self.conversion_op::<_, false, false>(frame, def_id, |value| {
                    ConversionOperation::InstanceOf {
                        value,
                        target_type: target_type.clone(),
                    }
                })?
            }
            MonitorEnter => {
                let object_ref = frame.pop_value()?;
                let monitor_op = MonitorOperation::Enter(object_ref);
                IR::SideEffect {
                    rhs: Expression::Monitor(monitor_op),
                }
            }
            MonitorExit => {
                let object_ref = frame.pop_value()?;
                let monitor_op = MonitorOperation::Exit(object_ref);
                IR::SideEffect {
                    rhs: Expression::Monitor(monitor_op),
                }
            }
            WideILoad(idx) | WideFLoad(idx) | WideALoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value)?;
                IR::Nop
            }
            WideLLoad(idx) | WideDLoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value)?;
                frame.push_padding()?;
                IR::Nop
            }
            WideIStore(idx) | WideFStore(idx) | WideAStore(idx) => {
                let value = frame.pop_value()?;
                frame.set_local(*idx, value)?;
                IR::Nop
            }
            WideLStore(idx) | WideDStore(idx) => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;
                frame.set_local(*idx, value)?;
                frame.set_local_padding(idx + 1)?;
                IR::Nop
            }
            Breakpoint | ImpDep1 | ImpDep2 => IR::Nop,
        };
        self.ir_instructions.insert(pc, ir_instruction);

        Ok(())
    }

    fn unitary_conditional_jump<C>(
        &mut self,
        frame: &mut StackFrame,
        target: ProgramCounter,
        condition: C,
    ) -> Result<IR, MokaIRGenerationError>
    where
        C: FnOnce(ValueRef) -> Condition,
    {
        let operand = frame.pop_value()?;
        Ok(IR::Jump {
            condition: Some(condition(operand)),
            target,
        })
    }

    fn binary_conditional_jump<C>(
        &mut self,
        frame: &mut StackFrame,
        target: ProgramCounter,
        condition: C,
    ) -> Result<IR, MokaIRGenerationError>
    where
        C: FnOnce(ValueRef, ValueRef) -> Condition,
    {
        let lhs = frame.pop_value()?;
        let rhs = frame.pop_value()?;
        Ok(IR::Jump {
            condition: Some(condition(lhs, rhs)),
            target,
        })
    }

    fn conversion_op<C, const OPERAND_WIDE: bool, const RESULT_WIDE: bool>(
        &mut self,
        frame: &mut StackFrame,
        def_id: Identifier,
        conversion: C,
    ) -> Result<IR, MokaIRGenerationError>
    where
        C: FnOnce(ValueRef) -> ConversionOperation,
    {
        let operand = {
            if OPERAND_WIDE {
                frame.pop_padding()?;
            }
            frame.pop_value()?
        };
        frame.push_value(def_id.into())?;
        if RESULT_WIDE {
            frame.push_padding()?;
        }
        Ok(IR::Assignment {
            lhs: def_id,
            rhs: Expression::Conversion(conversion(operand)),
        })
    }

    fn binary_op_math<M>(
        &mut self,
        frame: &mut StackFrame,
        def_id: Identifier,
        math: M,
    ) -> Result<IR, MokaIRGenerationError>
    where
        M: FnOnce(ValueRef, ValueRef) -> MathOperation,
    {
        self.binary_op_assignment(frame, def_id, |lhs, rhs| Expression::Math(math(lhs, rhs)))
    }
    fn binary_wide_math<M>(
        &mut self,
        frame: &mut StackFrame,
        def_id: Identifier,
        math: M,
    ) -> Result<IR, MokaIRGenerationError>
    where
        M: FnOnce(ValueRef, ValueRef) -> MathOperation,
    {
        self.binary_wide_op_assignment(frame, def_id, |lhs, rhs| Expression::Math(math(lhs, rhs)))
    }

    fn binary_op_assignment<E>(
        &mut self,
        frame: &mut StackFrame,
        def_id: Identifier,
        expr: E,
    ) -> Result<IR, MokaIRGenerationError>
    where
        E: FnOnce(ValueRef, ValueRef) -> Expression,
    {
        let lhs = frame.pop_value()?;
        let rhs = frame.pop_value()?;
        frame.push_value(def_id.into())?;
        Ok(IR::Assignment {
            lhs: def_id,
            rhs: expr(lhs, rhs),
        })
    }

    fn binary_wide_op_assignment<E>(
        &mut self,
        frame: &mut StackFrame,
        def_id: Identifier,
        expr: E,
    ) -> Result<IR, MokaIRGenerationError>
    where
        E: FnOnce(ValueRef, ValueRef) -> Expression,
    {
        let lhs = {
            frame.pop_padding()?;
            frame.pop_value()?
        };
        let rhs = {
            frame.pop_padding()?;
            frame.pop_value()?
        };
        frame.push_value(def_id.into())?;
        frame.push_padding()?;
        Ok(IR::Assignment {
            lhs: def_id,
            rhs: expr(lhs, rhs),
        })
    }
}
