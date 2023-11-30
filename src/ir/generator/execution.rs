use super::{jvm_frame::JvmFrame, MokaIRGenerationError, MokaIRGenerator};
use crate::{
    elements::{
        instruction::{Instruction, ProgramCounter},
        references::TypeReference,
        ConstantValue, MethodDescriptor, ReturnType,
    },
    ir::{expressions::*, Argument, Condition, Expression, LocalDef, MokaInstruction as IR},
    types::FieldType,
};

use std::collections::LinkedList;

impl MokaIRGenerator<'_> {
    pub(super) fn run_instruction(
        &mut self,
        insn: &Instruction,
        pc: ProgramCounter,
        frame: &mut JvmFrame,
    ) -> Result<IR, MokaIRGenerationError> {
        use Instruction::*;
        let def_id = LocalDef::new(pc.into());
        let ir_instruction = match insn {
            Nop => IR::Nop,
            AConstNull => self.const_assignment(frame, def_id, ConstantValue::Null)?,
            IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => self
                .const_assignment(
                    frame,
                    def_id,
                    ConstantValue::Integer((insn.opcode() as i32) - 3),
                )?,
            LConst0 | LConst1 => self.wide_const_assignment(
                frame,
                def_id,
                ConstantValue::Long((insn.opcode() as i64) - 9),
            )?,
            FConst0 | FConst1 | FConst2 => self.const_assignment(
                frame,
                def_id,
                ConstantValue::Float((insn.opcode() as f32) - 11.0),
            )?,
            DConst0 | DConst1 => self.wide_const_assignment(
                frame,
                def_id,
                ConstantValue::Double((insn.opcode() as f64) - 14.0),
            )?,
            BiPush(value) => {
                self.const_assignment(frame, def_id, ConstantValue::Integer(*value as i32))?
            }
            SiPush(value) => {
                self.const_assignment(frame, def_id, ConstantValue::Integer(*value as i32))?
            }
            Ldc(value) | LdcW(value) => self.const_assignment(frame, def_id, value.clone())?,
            Ldc2W(value) => self.wide_const_assignment(frame, def_id, value.clone())?,
            ILoad(idx) | FLoad(idx) | ALoad(idx) => self.load_local(frame, *idx as u16)?,
            LLoad(idx) | DLoad(idx) => self.load_dual_slot_local(frame, *idx as u16)?,
            ILoad0 | FLoad0 | ALoad0 => self.load_local(frame, 0)?,
            ILoad1 | FLoad1 | ALoad1 => self.load_local(frame, 1)?,
            ILoad2 | FLoad2 | ALoad2 => self.load_local(frame, 2)?,
            ILoad3 | FLoad3 | ALoad3 => self.load_local(frame, 3)?,
            LLoad0 | DLoad0 => self.load_dual_slot_local(frame, 0)?,
            LLoad1 | DLoad1 => self.load_dual_slot_local(frame, 1)?,
            LLoad2 | DLoad2 => self.load_dual_slot_local(frame, 2)?,
            LLoad3 | DLoad3 => self.load_dual_slot_local(frame, 3)?,
            IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Read { array_ref, index };

                frame.push_value(Argument::Id(def_id.into()))?;
                IR::Definition {
                    def_id,
                    expr: Expression::Array(array_op),
                }
            }
            LALoad | DALoad => {
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Read { array_ref, index };

                frame.push_dual_slot_value(Argument::Id(def_id.into()))?;
                IR::Definition {
                    def_id,
                    expr: Expression::Array(array_op),
                }
            }
            IStore(idx) | FStore(idx) | AStore(idx) => self.store_local(frame, *idx as u16)?,
            LStore(idx) | DStore(idx) => self.store_dual_slot_local(frame, *idx as u16)?,
            IStore0 | FStore0 | AStore0 => self.store_local(frame, 0)?,
            IStore1 | FStore1 | AStore1 => self.store_local(frame, 1)?,
            IStore2 | FStore2 | AStore2 => self.store_local(frame, 2)?,
            IStore3 | FStore3 | AStore3 => self.store_local(frame, 3)?,
            LStore0 | DStore0 => self.store_dual_slot_local(frame, 0)?,
            LStore1 | DStore1 => self.store_dual_slot_local(frame, 1)?,
            LStore2 | DStore2 => self.store_dual_slot_local(frame, 2)?,
            LStore3 | DStore3 => self.store_dual_slot_local(frame, 3)?,
            IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
                let value = frame.pop_value()?;
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Write {
                    array_ref,
                    index,
                    value,
                };

                IR::Definition {
                    def_id,
                    expr: Expression::Array(array_op),
                }
            }
            LAStore | DAStore => {
                let value = frame.pop_dual_slot_value()?;
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Write {
                    array_ref,
                    index,
                    value,
                };
                IR::Definition {
                    def_id,
                    expr: Expression::Array(array_op),
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
                frame.push_value(def_id.into_value_ref())?;
                let math_op = MathOperation::Negate(value);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
                }
            }
            LNeg | DNeg => {
                let operand = frame.pop_dual_slot_value()?;
                frame.push_dual_slot_value(def_id.into_value_ref())?;
                let math_op = MathOperation::Negate(operand);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
                }
            }
            IShl => self.binary_op_math(frame, def_id, MathOperation::ShiftLeft)?,
            IShr => self.binary_op_math(frame, def_id, MathOperation::ShiftRight)?,
            LShl => {
                let shift_amount = frame.pop_value()?;
                let base = frame.pop_dual_slot_value()?;
                frame.push_dual_slot_value(def_id.into_value_ref())?;
                let math_op = MathOperation::ShiftLeft(base, shift_amount);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
                }
            }
            LShr => {
                let shift_amount = frame.pop_value()?;
                let base = frame.pop_dual_slot_value()?;
                frame.push_dual_slot_value(def_id.into_value_ref())?;
                let math_op = MathOperation::ShiftRight(base, shift_amount);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
                }
            }
            LUShr => {
                let shift_amount = frame.pop_value()?;
                let base = frame.pop_dual_slot_value()?;
                frame.push_dual_slot_value(def_id.into_value_ref())?;
                let math_op = MathOperation::LogicalShiftRight(base, shift_amount);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
                }
            }
            IUShr => self.binary_op_math(frame, def_id, MathOperation::LogicalShiftRight)?,
            IAnd => self.binary_op_math(frame, def_id, MathOperation::BitwiseAnd)?,
            LAnd => self.binary_wide_math(frame, def_id, MathOperation::BitwiseAnd)?,
            IOr => self.binary_op_math(frame, def_id, MathOperation::BitwiseOr)?,
            LOr => self.binary_wide_math(frame, def_id, MathOperation::BitwiseOr)?,
            IXor => self.binary_op_math(frame, def_id, MathOperation::BitwiseXor)?,
            LXor => self.binary_wide_math(frame, def_id, MathOperation::BitwiseXor)?,
            IInc(idx, _) => {
                let base = frame.get_local(*idx)?;
                frame.set_local(*idx, def_id.into_value_ref())?;
                let math_op = MathOperation::Increment(base);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
                }
            }
            WideIInc(idx, _) => {
                let base = frame.get_local(*idx)?;
                frame.set_local(*idx, def_id.into_value_ref())?;
                let math_op = MathOperation::Increment(base);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
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
                let lhs = frame.pop_dual_slot_value()?;
                let rhs = frame.pop_dual_slot_value()?;
                frame.push_value(def_id.into_value_ref())?;
                let math_op = MathOperation::LongComparison(lhs, rhs);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
                }
            }
            FCmpL | FCmpG => {
                let lhs = frame.pop_value()?;
                let rhs = frame.pop_value()?;
                frame.push_value(def_id.into_value_ref())?;
                let nan_treatment = match insn {
                    FCmpG => NaNTreatment::IsLargest,
                    FCmpL => NaNTreatment::IsSmallest,
                    _ => unreachable!(),
                };
                let math_op = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
                }
            }
            DCmpL | DCmpG => {
                let lhs = frame.pop_dual_slot_value()?;
                let rhs = frame.pop_dual_slot_value()?;
                frame.push_value(def_id.into_value_ref())?;
                let nan_treatment = match insn {
                    DCmpG => NaNTreatment::IsLargest,
                    DCmpL => NaNTreatment::IsSmallest,
                    _ => unreachable!(),
                };
                let math_op = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment);
                IR::Definition {
                    def_id,
                    expr: Expression::Math(math_op),
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
            Jsr(target) | JsrW(target) => {
                let next_pc = self.next_pc_of(pc)?;
                let value = Expression::Subroutine {
                    return_address: next_pc,
                    target: *target,
                };
                frame.push_value(def_id.into_value_ref())?;
                IR::Definition {
                    def_id,
                    expr: value,
                }
            }
            Ret(idx) => {
                let return_address = frame.get_local(*idx)?;
                IR::SubroutineRet(return_address)
            }
            WideRet(idx) => {
                let return_address = frame.get_local(*idx)?;
                IR::SubroutineRet(return_address)
            }
            TableSwitch {
                range,
                jump_targets,
                default,
            } => {
                let condition = frame.pop_value()?;
                let branches = range.clone().zip(jump_targets.clone()).collect();
                IR::Switch {
                    match_value: condition,
                    default: *default,
                    branches,
                }
            }
            LookupSwitch {
                default,
                match_targets,
            } => {
                let condition = frame.pop_value()?;
                IR::Switch {
                    match_value: condition,
                    default: *default,
                    branches: match_targets.clone(),
                }
            }
            IReturn | FReturn | AReturn => {
                let value = frame.pop_value()?;
                IR::Return(Some(value))
            }
            LReturn | DReturn => {
                let value = frame.pop_dual_slot_value()?;
                IR::Return(Some(value))
            }
            Return => IR::Return(None),
            GetStatic(field) => {
                frame.typed_push(&field.field_type, def_id.into_value_ref())?;
                let field_op = FieldAccess::ReadStatic {
                    field: field.clone(),
                };
                IR::Definition {
                    def_id,
                    expr: Expression::Field(field_op),
                }
            }
            GetField(field) => {
                let object_ref = frame.pop_value()?;
                frame.typed_push(&field.field_type, def_id.into_value_ref())?;
                let field_op = FieldAccess::ReadInstance {
                    object_ref,
                    field: field.clone(),
                };
                IR::Definition {
                    def_id,
                    expr: Expression::Field(field_op),
                }
            }
            PutStatic(field) => {
                let value = frame.typed_pop(&field.field_type)?;
                let field_op = FieldAccess::WriteStatic {
                    field: field.clone(),
                    value,
                };
                IR::Definition {
                    def_id,
                    expr: Expression::Field(field_op),
                }
            }
            PutField(field) => {
                let value = frame.typed_pop(&field.field_type)?;
                let object_ref = frame.pop_value()?;
                let field_op = FieldAccess::WriteInstance {
                    object_ref,
                    field: field.clone(),
                    value,
                };
                IR::Definition {
                    def_id,
                    expr: Expression::Field(field_op),
                }
            }
            InvokeVirtual(method_ref)
            | InvokeSpecial(method_ref)
            | InvokeInterface(method_ref, _) => {
                let arguments = self.pop_args(frame, &method_ref.descriptor)?;
                let object_ref = frame.pop_value()?;
                let rhs = Expression::Call(method_ref.clone(), Some(object_ref), arguments);
                if let ReturnType::Some(return_type) = &method_ref.descriptor.return_type {
                    frame.typed_push(return_type, def_id.into_value_ref())?;
                }
                IR::Definition { def_id, expr: rhs }
            }
            InvokeStatic(method_ref) => {
                let arguments = self.pop_args(frame, &method_ref.descriptor)?;
                let rhs = Expression::Call(method_ref.clone(), None, arguments);
                if let ReturnType::Some(return_type) = &method_ref.descriptor.return_type {
                    frame.typed_push(return_type, def_id.into_value_ref())?;
                }
                IR::Definition { def_id, expr: rhs }
            }
            InvokeDynamic {
                descriptor,
                bootstrap_method_index,
                name,
            } => {
                let arguments = self.pop_args(frame, descriptor)?;
                let rhs = Expression::GetClosure(
                    *bootstrap_method_index,
                    name.to_owned(),
                    arguments,
                    descriptor.to_owned(),
                );
                if let ReturnType::Some(return_type) = &descriptor.return_type {
                    frame.typed_push(&return_type, def_id.into_value_ref())?;
                }
                IR::Definition { def_id, expr: rhs }
            }
            New(class) => {
                frame.push_value(def_id.into_value_ref())?;
                IR::Definition {
                    def_id,
                    expr: Expression::New(class.clone()),
                }
            }
            ANewArray(class_ref) => {
                let count = frame.pop_value()?;
                frame.push_value(def_id.into_value_ref())?;
                let array_op = ArrayOperation::New {
                    element_type: FieldType::Object(class_ref.clone()),
                    length: count,
                };
                IR::Definition {
                    def_id,
                    expr: Expression::Array(array_op),
                }
            }
            NewArray(prim_type) => {
                let count = frame.pop_value()?;
                frame.push_value(def_id.into_value_ref())?;
                let array_op = ArrayOperation::New {
                    element_type: FieldType::Base(*prim_type),
                    length: count,
                };
                IR::Definition {
                    def_id,
                    expr: Expression::Array(array_op),
                }
            }
            MultiANewArray(TypeReference(element_type), dimension) => {
                let counts: Vec<_> = (0..*dimension)
                    .map(|_| frame.pop_value())
                    .collect::<Result<_, _>>()?;
                frame.push_value(def_id.into_value_ref())?;
                let array_op = ArrayOperation::NewMultiDim {
                    element_type: element_type.clone(),
                    dimensions: counts,
                };
                IR::Definition {
                    def_id,
                    expr: Expression::Array(array_op),
                }
            }
            ArrayLength => {
                let array_ref = frame.pop_value()?;
                frame.push_value(def_id.into_value_ref())?;
                let array_op = ArrayOperation::Length { array_ref };
                IR::Definition {
                    def_id,
                    expr: Expression::Array(array_op),
                }
            }
            AThrow => {
                let exception_ref = frame.pop_value()?;
                IR::Definition {
                    def_id,
                    expr: Expression::Throw(exception_ref),
                }
            }
            CheckCast(TypeReference(target_type)) => {
                self.conversion_op::<_, false, false>(frame, def_id, |value| {
                    ConversionOperation::CheckCast(value, target_type.clone())
                })?
            }
            InstanceOf(TypeReference(target_type)) => {
                self.conversion_op::<_, false, false>(frame, def_id, |value| {
                    ConversionOperation::InstanceOf(value, target_type.clone())
                })?
            }
            MonitorEnter => {
                let object_ref = frame.pop_value()?;
                let monitor_op = LockOperation::Acquire(object_ref);
                IR::Definition {
                    def_id,
                    expr: Expression::Synchronization(monitor_op),
                }
            }
            MonitorExit => {
                let object_ref = frame.pop_value()?;
                let monitor_op = LockOperation::Release(object_ref);
                IR::Definition {
                    def_id,
                    expr: Expression::Synchronization(monitor_op),
                }
            }
            WideILoad(idx) | WideFLoad(idx) | WideALoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value)?;
                IR::Nop
            }
            WideLLoad(idx) | WideDLoad(idx) => {
                let value = frame.get_dual_slot_local(*idx)?;
                frame.push_dual_slot_value(value)?;
                IR::Nop
            }
            WideIStore(idx) | WideFStore(idx) | WideAStore(idx) => {
                let value = frame.pop_value()?;
                frame.set_local(*idx, value)?;
                IR::Nop
            }
            WideLStore(idx) | WideDStore(idx) => {
                let value = frame.pop_dual_slot_value()?;
                frame.set_dual_slot_local(*idx, value)?;
                IR::Nop
            }
            Breakpoint | ImpDep1 | ImpDep2 => IR::Nop,
        };
        Ok(ir_instruction)
    }

    #[inline]
    fn pop_args(
        &mut self,
        frame: &mut JvmFrame,
        descriptor: &MethodDescriptor,
    ) -> Result<Vec<Argument>, MokaIRGenerationError> {
        let mut args = LinkedList::new();
        for param_type in descriptor.parameters_types.iter().rev() {
            let arg = frame.typed_pop(param_type)?;
            args.push_front(arg);
        }
        Ok(args.into_iter().collect())
    }

    #[inline]
    fn store_dual_slot_local(
        &mut self,
        frame: &mut JvmFrame,
        idx: u16,
    ) -> Result<IR, MokaIRGenerationError> {
        let value = frame.pop_dual_slot_value()?;
        frame.set_dual_slot_local(idx, value)?;
        Ok(IR::Nop)
    }

    #[inline]
    fn store_local(&mut self, frame: &mut JvmFrame, idx: u16) -> Result<IR, MokaIRGenerationError> {
        let value = frame.pop_value()?;
        frame.set_local(idx, value)?;
        Ok(IR::Nop)
    }

    #[inline]
    fn load_dual_slot_local(
        &mut self,
        frame: &mut JvmFrame,
        idx: u16,
    ) -> Result<IR, MokaIRGenerationError> {
        let value = frame.get_dual_slot_local(idx)?;
        frame.push_dual_slot_value(value)?;
        Ok(IR::Nop)
    }

    #[inline]
    fn load_local(&mut self, frame: &mut JvmFrame, idx: u16) -> Result<IR, MokaIRGenerationError> {
        let value = frame.get_local(idx)?;
        frame.push_value(value)?;
        Ok(IR::Nop)
    }

    #[inline]
    fn const_assignment(
        &mut self,
        frame: &mut JvmFrame,
        def_id: LocalDef,
        constant: ConstantValue,
    ) -> Result<IR, MokaIRGenerationError> {
        frame.push_value(def_id.into_value_ref())?;
        Ok(IR::Definition {
            def_id,
            expr: Expression::Const(constant),
        })
    }

    #[inline]
    fn wide_const_assignment(
        &mut self,
        frame: &mut JvmFrame,
        def_id: LocalDef,
        constant: ConstantValue,
    ) -> Result<IR, MokaIRGenerationError> {
        frame.push_dual_slot_value(def_id.into_value_ref())?;
        Ok(IR::Definition {
            def_id,
            expr: Expression::Const(constant),
        })
    }

    #[inline]
    fn unitary_conditional_jump<C>(
        &mut self,
        frame: &mut JvmFrame,
        target: ProgramCounter,
        condition: C,
    ) -> Result<IR, MokaIRGenerationError>
    where
        C: FnOnce(Argument) -> Condition,
    {
        let operand = frame.pop_value()?;
        Ok(IR::Jump {
            condition: Some(condition(operand)),
            target,
        })
    }

    #[inline]
    fn binary_conditional_jump<C>(
        &mut self,
        frame: &mut JvmFrame,
        target: ProgramCounter,
        condition: C,
    ) -> Result<IR, MokaIRGenerationError>
    where
        C: FnOnce(Argument, Argument) -> Condition,
    {
        let lhs = frame.pop_value()?;
        let rhs = frame.pop_value()?;
        Ok(IR::Jump {
            condition: Some(condition(lhs, rhs)),
            target,
        })
    }

    #[inline]
    fn conversion_op<C, const OPERAND_WIDE: bool, const RESULT_WIDE: bool>(
        &mut self,
        frame: &mut JvmFrame,
        def_id: LocalDef,
        conversion: C,
    ) -> Result<IR, MokaIRGenerationError>
    where
        C: FnOnce(Argument) -> ConversionOperation,
    {
        let operand = if OPERAND_WIDE {
            frame.pop_dual_slot_value()?
        } else {
            frame.pop_value()?
        };
        if RESULT_WIDE {
            frame.push_dual_slot_value(def_id.into_value_ref())?;
        } else {
            frame.push_value(def_id.into_value_ref())?;
        }
        Ok(IR::Definition {
            def_id,
            expr: Expression::Conversion(conversion(operand)),
        })
    }

    #[inline]
    fn binary_op_math<M>(
        &mut self,
        frame: &mut JvmFrame,
        def_id: LocalDef,
        math: M,
    ) -> Result<IR, MokaIRGenerationError>
    where
        M: FnOnce(Argument, Argument) -> MathOperation,
    {
        self.binary_op_assignment(frame, def_id, |lhs, rhs| Expression::Math(math(lhs, rhs)))
    }

    #[inline]
    fn binary_wide_math<M>(
        &mut self,
        frame: &mut JvmFrame,
        def_id: LocalDef,
        math: M,
    ) -> Result<IR, MokaIRGenerationError>
    where
        M: FnOnce(Argument, Argument) -> MathOperation,
    {
        self.binary_dual_slot_op_assignment(frame, def_id, |lhs, rhs| {
            Expression::Math(math(lhs, rhs))
        })
    }

    #[inline]
    fn binary_op_assignment<E>(
        &mut self,
        frame: &mut JvmFrame,
        def_id: LocalDef,
        expr: E,
    ) -> Result<IR, MokaIRGenerationError>
    where
        E: FnOnce(Argument, Argument) -> Expression,
    {
        let lhs = frame.pop_value()?;
        let rhs = frame.pop_value()?;
        frame.push_value(def_id.into_value_ref())?;
        Ok(IR::Definition {
            def_id,
            expr: expr(lhs, rhs),
        })
    }

    #[inline]
    fn binary_dual_slot_op_assignment<E>(
        &mut self,
        frame: &mut JvmFrame,
        def_id: LocalDef,
        expr: E,
    ) -> Result<IR, MokaIRGenerationError>
    where
        E: FnOnce(Argument, Argument) -> Expression,
    {
        let lhs = frame.pop_dual_slot_value()?;
        let rhs = frame.pop_dual_slot_value()?;
        frame.push_dual_slot_value(def_id.into_value_ref())?;
        Ok(IR::Definition {
            def_id,
            expr: expr(lhs, rhs),
        })
    }
}
