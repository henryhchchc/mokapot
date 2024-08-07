use super::{jvm_frame::JvmStackFrame, MokaIRBrewingError, MokaIRGenerator};
use crate::{
    ir::{
        expression::{
            ArrayOperation, Condition, Conversion, Expression, FieldAccess, LockOperation,
            MathOperation, NaNTreatment,
        },
        LocalValue, MokaInstruction as IR, Operand,
    },
    jvm::{
        code::{Instruction, ProgramCounter, WideInstruction},
        ConstantValue,
    },
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::ReturnType,
    },
};

#[allow(clippy::too_many_lines)]
impl MokaIRGenerator<'_> {
    pub(super) fn run_instruction(
        &mut self,
        insn: &Instruction,
        pc: ProgramCounter,
        frame: &mut JvmStackFrame,
    ) -> Result<IR, MokaIRBrewingError> {
        #[allow(clippy::enum_glob_use)]
        use Instruction::*;

        let def = LocalValue::new(pc.into());
        let ir_instruction = match insn {
            Nop | Breakpoint | ImpDep1 | ImpDep2 => IR::Nop,
            AConstNull => {
                frame.push_value(def.as_argument())?;
                let expr = Expression::Const(ConstantValue::Null);
                IR::Definition { value: def, expr }
            }
            IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
                frame.push_value(def.as_argument())?;
                let int_value = i32::from(insn.opcode()) - 3;
                let expr = Expression::Const(ConstantValue::Integer(int_value));
                IR::Definition { value: def, expr }
            }
            LConst0 | LConst1 => {
                frame.push_dual_slot_value(def.as_argument())?;
                let long_value = i64::from(insn.opcode()) - 9;
                let expr = Expression::Const(ConstantValue::Long(long_value));
                IR::Definition { value: def, expr }
            }
            FConst0 | FConst1 | FConst2 => {
                frame.push_value(def.as_argument())?;
                let float_value = f32::from(insn.opcode()) - 11.0;
                let expr = Expression::Const(ConstantValue::Float(float_value));
                IR::Definition { value: def, expr }
            }
            DConst0 | DConst1 => {
                frame.push_dual_slot_value(def.as_argument())?;
                let double_value = f64::from(insn.opcode()) - 14.0;
                let expr = Expression::Const(ConstantValue::Double(double_value));
                IR::Definition { value: def, expr }
            }
            BiPush(value) => {
                frame.push_value(def.as_argument())?;
                let expr = Expression::Const(ConstantValue::Integer(i32::from(*value)));
                IR::Definition { value: def, expr }
            }
            SiPush(value) => {
                frame.push_value(def.as_argument())?;
                let expr = Expression::Const(ConstantValue::Integer(i32::from(*value)));
                IR::Definition { value: def, expr }
            }
            Ldc(value) | LdcW(value) => {
                frame.push_value(def.as_argument())?;
                let expr = Expression::Const(value.clone());
                IR::Definition { value: def, expr }
            }
            Ldc2W(value) => {
                frame.push_dual_slot_value(def.as_argument())?;
                let expr = Expression::Const(value.clone());
                IR::Definition { value: def, expr }
            }
            ILoad(idx) | FLoad(idx) | ALoad(idx) => load_local(frame, u16::from(*idx))?,
            LLoad(idx) | DLoad(idx) => load_dual_slot_local(frame, u16::from(*idx))?,
            ILoad0 | FLoad0 | ALoad0 => load_local(frame, 0)?,
            ILoad1 | FLoad1 | ALoad1 => load_local(frame, 1)?,
            ILoad2 | FLoad2 | ALoad2 => load_local(frame, 2)?,
            ILoad3 | FLoad3 | ALoad3 => load_local(frame, 3)?,
            LLoad0 | DLoad0 => load_dual_slot_local(frame, 0)?,
            LLoad1 | DLoad1 => load_dual_slot_local(frame, 1)?,
            LLoad2 | DLoad2 => load_dual_slot_local(frame, 2)?,
            LLoad3 | DLoad3 => load_dual_slot_local(frame, 3)?,
            IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Read { array_ref, index };

                frame.push_value(Operand::Just(def.into()))?;
                IR::Definition {
                    value: def,
                    expr: Expression::Array(array_op),
                }
            }
            LALoad | DALoad => {
                let index = frame.pop_value()?;
                let array_ref = frame.pop_value()?;
                let array_op = ArrayOperation::Read { array_ref, index };

                frame.push_dual_slot_value(Operand::Just(def.into()))?;
                IR::Definition {
                    value: def,
                    expr: Expression::Array(array_op),
                }
            }
            IStore(idx) | FStore(idx) | AStore(idx) => store_local(frame, u16::from(*idx))?,
            LStore(idx) | DStore(idx) => store_dual_slot_local(frame, u16::from(*idx))?,
            IStore0 | FStore0 | AStore0 => store_local(frame, 0)?,
            IStore1 | FStore1 | AStore1 => store_local(frame, 1)?,
            IStore2 | FStore2 | AStore2 => store_local(frame, 2)?,
            IStore3 | FStore3 | AStore3 => store_local(frame, 3)?,
            LStore0 | DStore0 => store_dual_slot_local(frame, 0)?,
            LStore1 | DStore1 => store_dual_slot_local(frame, 1)?,
            LStore2 | DStore2 => store_dual_slot_local(frame, 2)?,
            LStore3 | DStore3 => store_dual_slot_local(frame, 3)?,
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
                    value: def,
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
                    value: def,
                    expr: Expression::Array(array_op),
                }
            }
            Pop => {
                frame.pop()?;
                IR::Nop
            }
            Pop2 => {
                frame.pop2()?;
                IR::Nop
            }
            Dup => {
                frame.dup()?;
                IR::Nop
            }
            DupX1 => {
                frame.dup_x1()?;
                IR::Nop
            }
            DupX2 => {
                frame.dup_x2()?;
                IR::Nop
            }
            Dup2 => {
                frame.dup2()?;
                IR::Nop
            }
            Dup2X1 => {
                frame.dup2_x1()?;
                IR::Nop
            }
            Dup2X2 => {
                frame.dup2_x2()?;
                IR::Nop
            }
            Swap => {
                frame.swap()?;
                IR::Nop
            }
            IAdd | FAdd => binary_op_math(frame, def, MathOperation::Add)?,
            LAdd | DAdd => binary_wide_math(frame, def, MathOperation::Add)?,
            ISub | FSub => binary_op_math(frame, def, MathOperation::Subtract)?,
            LSub | DSub => binary_wide_math(frame, def, MathOperation::Subtract)?,
            IMul | FMul => binary_op_math(frame, def, MathOperation::Multiply)?,
            LMul | DMul => binary_wide_math(frame, def, MathOperation::Multiply)?,
            IDiv | FDiv => binary_op_math(frame, def, MathOperation::Divide)?,
            LDiv | DDiv => binary_wide_math(frame, def, MathOperation::Divide)?,
            IRem | FRem => binary_op_math(frame, def, MathOperation::Remainder)?,
            LRem | DRem => binary_wide_math(frame, def, MathOperation::Remainder)?,
            INeg | FNeg => {
                let value = frame.pop_value()?;
                frame.push_value(def.as_argument())?;
                let math_op = MathOperation::Negate(value);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            LNeg | DNeg => {
                let operand = frame.pop_dual_slot_value()?;
                frame.push_dual_slot_value(def.as_argument())?;
                let math_op = MathOperation::Negate(operand);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            IShl => binary_op_math(frame, def, MathOperation::ShiftLeft)?,
            IShr => binary_op_math(frame, def, MathOperation::ShiftRight)?,
            LShl => {
                let shift_amount = frame.pop_value()?;
                let base = frame.pop_dual_slot_value()?;
                frame.push_dual_slot_value(def.as_argument())?;
                let math_op = MathOperation::ShiftLeft(base, shift_amount);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            LShr => {
                let shift_amount = frame.pop_value()?;
                let base = frame.pop_dual_slot_value()?;
                frame.push_dual_slot_value(def.as_argument())?;
                let math_op = MathOperation::ShiftRight(base, shift_amount);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            LUShr => {
                let shift_amount = frame.pop_value()?;
                let base = frame.pop_dual_slot_value()?;
                frame.push_dual_slot_value(def.as_argument())?;
                let math_op = MathOperation::LogicalShiftRight(base, shift_amount);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            IUShr => binary_op_math(frame, def, MathOperation::LogicalShiftRight)?,
            IAnd => binary_op_math(frame, def, MathOperation::BitwiseAnd)?,
            LAnd => binary_wide_math(frame, def, MathOperation::BitwiseAnd)?,
            IOr => binary_op_math(frame, def, MathOperation::BitwiseOr)?,
            LOr => binary_wide_math(frame, def, MathOperation::BitwiseOr)?,
            IXor => binary_op_math(frame, def, MathOperation::BitwiseXor)?,
            LXor => binary_wide_math(frame, def, MathOperation::BitwiseXor)?,
            IInc(idx, constant) => {
                let base = frame.get_local(*idx)?;
                frame.set_local(*idx, def.as_argument())?;
                let math_op = MathOperation::Increment(base, *constant);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            Wide(WideInstruction::IInc(idx, constant)) => {
                let base = frame.get_local(*idx)?;
                frame.set_local(*idx, def.as_argument())?;
                let math_op = MathOperation::Increment(base, *constant);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            I2F => conversion_op::<_, false, false>(frame, def, Conversion::Int2Float)?,
            I2L => conversion_op::<_, false, true>(frame, def, Conversion::Int2Long)?,
            I2D => conversion_op::<_, false, true>(frame, def, Conversion::Int2Double)?,
            L2I => conversion_op::<_, true, false>(frame, def, Conversion::Long2Int)?,
            L2F => conversion_op::<_, true, false>(frame, def, Conversion::Long2Float)?,
            L2D => conversion_op::<_, true, true>(frame, def, Conversion::Long2Double)?,
            F2I => conversion_op::<_, false, false>(frame, def, Conversion::Float2Int)?,
            F2L => conversion_op::<_, false, true>(frame, def, Conversion::Float2Long)?,
            F2D => conversion_op::<_, false, true>(frame, def, Conversion::Float2Double)?,
            D2I => conversion_op::<_, true, false>(frame, def, Conversion::Double2Int)?,
            D2L => conversion_op::<_, true, true>(frame, def, Conversion::Double2Long)?,
            D2F => conversion_op::<_, true, false>(frame, def, Conversion::Double2Float)?,
            I2B => conversion_op::<_, false, false>(frame, def, Conversion::Int2Byte)?,
            I2C => conversion_op::<_, false, false>(frame, def, Conversion::Int2Char)?,
            I2S => conversion_op::<_, false, false>(frame, def, Conversion::Int2Short)?,
            LCmp => {
                let rhs = frame.pop_dual_slot_value()?;
                let lhs = frame.pop_dual_slot_value()?;
                frame.push_value(def.as_argument())?;
                let math_op = MathOperation::LongComparison(lhs, rhs);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            FCmpL | FCmpG => {
                let rhs = frame.pop_value()?;
                let lhs = frame.pop_value()?;
                frame.push_value(def.as_argument())?;
                let nan_treatment = match insn {
                    FCmpG => NaNTreatment::IsLargest,
                    FCmpL => NaNTreatment::IsSmallest,
                    _ => unreachable!(),
                };
                let math_op = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            DCmpL | DCmpG => {
                let rhs = frame.pop_dual_slot_value()?;
                let lhs = frame.pop_dual_slot_value()?;
                frame.push_value(def.as_argument())?;
                let nan_treatment = match insn {
                    DCmpG => NaNTreatment::IsLargest,
                    DCmpL => NaNTreatment::IsSmallest,
                    _ => unreachable!(),
                };
                let math_op = MathOperation::FloatingPointComparison(lhs, rhs, nan_treatment);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            IfEq(target) => unitary_conditional_jump(frame, *target, Condition::IsZero)?,
            IfNe(target) => unitary_conditional_jump(frame, *target, Condition::IsNonZero)?,
            IfLt(target) => unitary_conditional_jump(frame, *target, Condition::IsNegative)?,
            IfGe(target) => unitary_conditional_jump(frame, *target, Condition::IsNonNegative)?,
            IfGt(target) => unitary_conditional_jump(frame, *target, Condition::IsPositive)?,
            IfLe(target) => unitary_conditional_jump(frame, *target, Condition::IsNonPositive)?,
            IfNull(target) => unitary_conditional_jump(frame, *target, Condition::IsNull)?,
            IfNonNull(target) => unitary_conditional_jump(frame, *target, Condition::IsNotNull)?,
            IfICmpEq(target) | IfACmpEq(target) => {
                binary_conditional_jump(frame, *target, Condition::Equal)?
            }
            IfICmpNe(target) | IfACmpNe(target) => {
                binary_conditional_jump(frame, *target, Condition::NotEqual)?
            }
            IfICmpGe(target) => {
                binary_conditional_jump(frame, *target, Condition::GreaterThanOrEqual)?
            }
            IfICmpLt(target) => binary_conditional_jump(frame, *target, Condition::LessThan)?,
            IfICmpGt(target) => binary_conditional_jump(frame, *target, Condition::GreaterThan)?,
            IfICmpLe(target) => {
                binary_conditional_jump(frame, *target, Condition::LessThanOrEqual)?
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
                frame.push_value(def.as_argument())?;
                IR::Definition {
                    value: def,
                    expr: value,
                }
            }
            Ret(idx) => {
                let return_address = frame.get_local(*idx)?;
                IR::SubroutineRet(return_address)
            }
            Wide(WideInstruction::Ret(idx)) => {
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
                frame.typed_push(&field.field_type, def.as_argument())?;
                let field_op = FieldAccess::ReadStatic {
                    field: field.clone(),
                };
                IR::Definition {
                    value: def,
                    expr: Expression::Field(field_op),
                }
            }
            GetField(field) => {
                let object_ref = frame.pop_value()?;
                frame.typed_push(&field.field_type, def.as_argument())?;
                let field_op = FieldAccess::ReadInstance {
                    object_ref,
                    field: field.clone(),
                };
                IR::Definition {
                    value: def,
                    expr: Expression::Field(field_op),
                }
            }
            PutStatic(field) => {
                use PrimitiveType::{Double, Long};
                let value = if let FieldType::Base(Double | Long) = field.field_type {
                    frame.pop_dual_slot_value()?
                } else {
                    frame.pop_value()?
                };
                let field_op = FieldAccess::WriteStatic {
                    field: field.clone(),
                    value,
                };
                IR::Definition {
                    value: def,
                    expr: Expression::Field(field_op),
                }
            }
            PutField(field) => {
                use PrimitiveType::{Double, Long};
                let value = if let FieldType::Base(Double | Long) = field.field_type {
                    frame.pop_dual_slot_value()?
                } else {
                    frame.pop_value()?
                };
                let object_ref = frame.pop_value()?;
                let field_op = FieldAccess::WriteInstance {
                    object_ref,
                    field: field.clone(),
                    value,
                };
                IR::Definition {
                    value: def,
                    expr: Expression::Field(field_op),
                }
            }
            InvokeVirtual(method_ref)
            | InvokeSpecial(method_ref)
            | InvokeInterface(method_ref, _) => {
                let arguments = frame.pop_args(&method_ref.descriptor)?;
                let object_ref = frame.pop_value()?;
                let rhs = Expression::Call {
                    method: method_ref.clone(),
                    this: Some(object_ref),
                    args: arguments,
                };
                if let ReturnType::Some(ref return_type) = &method_ref.descriptor.return_type {
                    frame.typed_push(return_type, def.as_argument())?;
                }
                IR::Definition {
                    value: def,
                    expr: rhs,
                }
            }
            InvokeStatic(method_ref) => {
                let arguments = frame.pop_args(&method_ref.descriptor)?;
                let rhs = Expression::Call {
                    method: method_ref.clone(),
                    this: None,
                    args: arguments,
                };
                if let ReturnType::Some(ref return_type) = &method_ref.descriptor.return_type {
                    frame.typed_push(return_type, def.as_argument())?;
                }
                IR::Definition {
                    value: def,
                    expr: rhs,
                }
            }
            InvokeDynamic {
                descriptor,
                bootstrap_method_index,
                name,
            } => {
                let arguments = frame.pop_args(descriptor)?;
                let rhs = Expression::Closure {
                    bootstrap_method_index: *bootstrap_method_index,
                    name: name.to_owned(),
                    captures: arguments,
                    closure_descriptor: descriptor.to_owned(),
                };
                if let ReturnType::Some(return_type) = &descriptor.return_type {
                    frame.typed_push(return_type, def.as_argument())?;
                }
                IR::Definition {
                    value: def,
                    expr: rhs,
                }
            }
            New(class) => {
                frame.push_value(def.as_argument())?;
                IR::Definition {
                    value: def,
                    expr: Expression::New(class.clone()),
                }
            }
            ANewArray(class_ref) => {
                let count = frame.pop_value()?;
                frame.push_value(def.as_argument())?;
                let array_op = ArrayOperation::New {
                    element_type: FieldType::Object(class_ref.clone()),
                    length: count,
                };
                IR::Definition {
                    value: def,
                    expr: Expression::Array(array_op),
                }
            }
            NewArray(prim_type) => {
                let count = frame.pop_value()?;
                frame.push_value(def.as_argument())?;
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
                    .map(|_| frame.pop_value())
                    .collect::<Result<_, _>>()?;
                frame.push_value(def.as_argument())?;
                let expr = Expression::Array(ArrayOperation::NewMultiDim {
                    element_type: element_type.clone(),
                    dimensions: counts,
                });
                IR::Definition { value: def, expr }
            }
            ArrayLength => {
                let array_ref = frame.pop_value()?;
                frame.push_value(def.as_argument())?;
                let expr = Expression::Array(ArrayOperation::Length { array_ref });
                IR::Definition { value: def, expr }
            }
            AThrow => {
                let exception_ref = frame.pop_value()?;
                let expr = Expression::Throw(exception_ref);
                IR::Definition { value: def, expr }
            }
            CheckCast(target_type) => conversion_op::<_, false, false>(frame, def, |value| {
                Conversion::CheckCast(value, target_type.clone())
            })?,
            InstanceOf(target_type) => conversion_op::<_, false, false>(frame, def, |value| {
                Conversion::InstanceOf(value, target_type.clone())
            })?,
            MonitorEnter => {
                let object_ref = frame.pop_value()?;
                let monitor_op = LockOperation::Acquire(object_ref);
                let expr = Expression::Synchronization(monitor_op);
                IR::Definition { value: def, expr }
            }
            MonitorExit => {
                let object_ref = frame.pop_value()?;
                let monitor_op = LockOperation::Release(object_ref);
                let expr = Expression::Synchronization(monitor_op);
                IR::Definition { value: def, expr }
            }
            Wide(
                WideInstruction::ILoad(idx)
                | WideInstruction::FLoad(idx)
                | WideInstruction::ALoad(idx),
            ) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value)?;
                IR::Nop
            }
            Wide(WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx)) => {
                let value = frame.get_dual_slot_local(*idx)?;
                frame.push_dual_slot_value(value)?;
                IR::Nop
            }
            Wide(
                WideInstruction::IStore(idx)
                | WideInstruction::FStore(idx)
                | WideInstruction::AStore(idx),
            ) => {
                let value = frame.pop_value()?;
                frame.set_local(*idx, value)?;
                IR::Nop
            }
            Wide(WideInstruction::LStore(idx) | WideInstruction::DStore(idx)) => {
                let value = frame.pop_dual_slot_value()?;
                frame.set_dual_slot_local(*idx, value)?;
                IR::Nop
            }
        };
        Ok(ir_instruction)
    }
}

#[inline]
fn store_dual_slot_local(frame: &mut JvmStackFrame, idx: u16) -> Result<IR, MokaIRBrewingError> {
    let value = frame.pop_dual_slot_value()?;
    frame.set_dual_slot_local(idx, value)?;
    Ok(IR::Nop)
}

#[inline]
fn store_local(frame: &mut JvmStackFrame, idx: u16) -> Result<IR, MokaIRBrewingError> {
    let value = frame.pop_value()?;
    frame.set_local(idx, value)?;
    Ok(IR::Nop)
}

#[inline]
fn load_dual_slot_local(frame: &mut JvmStackFrame, idx: u16) -> Result<IR, MokaIRBrewingError> {
    let value = frame.get_dual_slot_local(idx)?;
    frame.push_dual_slot_value(value)?;
    Ok(IR::Nop)
}

#[inline]
fn load_local(frame: &mut JvmStackFrame, idx: u16) -> Result<IR, MokaIRBrewingError> {
    let value = frame.get_local(idx)?;
    frame.push_value(value)?;
    Ok(IR::Nop)
}

#[inline]
fn unitary_conditional_jump<C>(
    frame: &mut JvmStackFrame,
    target: ProgramCounter,
    condition: C,
) -> Result<IR, MokaIRBrewingError>
where
    C: FnOnce(Operand) -> Condition,
{
    let operand = frame.pop_value()?;
    Ok(IR::Jump {
        condition: Some(condition(operand)),
        target,
    })
}

#[inline]
fn binary_conditional_jump<C>(
    frame: &mut JvmStackFrame,
    target: ProgramCounter,
    condition: C,
) -> Result<IR, MokaIRBrewingError>
where
    C: FnOnce(Operand, Operand) -> Condition,
{
    let rhs = frame.pop_value()?;
    let lhs = frame.pop_value()?;
    Ok(IR::Jump {
        condition: Some(condition(lhs, rhs)),
        target,
    })
}

#[inline]
fn conversion_op<C, const OPERAND_WIDE: bool, const RESULT_WIDE: bool>(
    frame: &mut JvmStackFrame,
    def_id: LocalValue,
    conversion: C,
) -> Result<IR, MokaIRBrewingError>
where
    C: FnOnce(Operand) -> Conversion,
{
    let operand = if OPERAND_WIDE {
        frame.pop_dual_slot_value()?
    } else {
        frame.pop_value()?
    };
    if RESULT_WIDE {
        frame.push_dual_slot_value(def_id.as_argument())?;
    } else {
        frame.push_value(def_id.as_argument())?;
    }
    Ok(IR::Definition {
        value: def_id,
        expr: Expression::Conversion(conversion(operand)),
    })
}

#[inline]
fn binary_op_math<M>(
    frame: &mut JvmStackFrame,
    def_id: LocalValue,
    math: M,
) -> Result<IR, MokaIRBrewingError>
where
    M: FnOnce(Operand, Operand) -> MathOperation,
{
    let rhs = frame.pop_value()?;
    let lhs = frame.pop_value()?;
    frame.push_value(def_id.as_argument())?;
    let expr = Expression::Math(math(lhs, rhs));
    Ok(IR::Definition {
        value: def_id,
        expr,
    })
}

#[inline]
fn binary_wide_math<M>(
    frame: &mut JvmStackFrame,
    def_id: LocalValue,
    math: M,
) -> Result<IR, MokaIRBrewingError>
where
    M: FnOnce(Operand, Operand) -> MathOperation,
{
    let rhs = frame.pop_dual_slot_value()?;
    let lhs = frame.pop_dual_slot_value()?;
    frame.push_dual_slot_value(def_id.as_argument())?;
    let expr = Expression::Math(math(lhs, rhs));
    Ok(IR::Definition {
        value: def_id,
        expr,
    })
}
