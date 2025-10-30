use super::{
    MokaIRBrewingError, MokaIRGenerator,
    jvm_frame::{DUAL_SLOT, JvmStackFrame, SINGLE_SLOT, SlotWidth},
};
use crate::{
    ir::{
        LocalValue, MokaInstruction as IR, Operand,
        expression::{
            ArrayOperation, Condition, Conversion, Expression, FieldAccess, LockOperation,
            MathOperation, NaNTreatment,
        },
        generator::jvm_frame::StackOperations,
    },
    jvm::{
        ConstantValue,
        code::{Instruction, ProgramCounter, WideInstruction},
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
        jvm_instruction: &Instruction,
        pc: ProgramCounter,
        frame: &mut JvmStackFrame,
    ) -> Result<IR, MokaIRBrewingError> {
        #[allow(clippy::enum_glob_use)]
        use Instruction::*;

        let def = LocalValue::new(pc.into());
        let ir_instruction = match jvm_instruction {
            Nop | Breakpoint | ImpDep1 | ImpDep2 => IR::Nop,
            AConstNull => {
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let expr = Expression::Const(ConstantValue::Null);
                IR::Definition { value: def, expr }
            }
            IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let int_value = i32::from(jvm_instruction.opcode()) - 3;
                let expr = Expression::Const(ConstantValue::Integer(int_value));
                IR::Definition { value: def, expr }
            }
            LConst0 | LConst1 => {
                let value = def.as_operand();
                frame.push_value::<DUAL_SLOT>(value)?;
                let long_value = i64::from(jvm_instruction.opcode()) - 9;
                let expr = Expression::Const(ConstantValue::Long(long_value));
                IR::Definition { value: def, expr }
            }
            FConst0 | FConst1 | FConst2 => {
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let float_value = f32::from(jvm_instruction.opcode()) - 11.0;
                let expr = Expression::Const(ConstantValue::Float(float_value));
                IR::Definition { value: def, expr }
            }
            DConst0 | DConst1 => {
                let value = def.as_operand();
                frame.push_value::<DUAL_SLOT>(value)?;
                let double_value = f64::from(jvm_instruction.opcode()) - 14.0;
                let expr = Expression::Const(ConstantValue::Double(double_value));
                IR::Definition { value: def, expr }
            }
            BiPush(value) => {
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let expr = Expression::Const(ConstantValue::Integer(i32::from(*value)));
                IR::Definition { value: def, expr }
            }
            SiPush(value) => {
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let expr = Expression::Const(ConstantValue::Integer(i32::from(*value)));
                IR::Definition { value: def, expr }
            }
            Ldc(value) | LdcW(value) => {
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let expr = Expression::Const(value.clone());
                IR::Definition { value: def, expr }
            }
            Ldc2W(value) => {
                frame.push_value::<DUAL_SLOT>(def.as_operand())?;
                let expr = Expression::Const(value.clone());
                IR::Definition { value: def, expr }
            }
            ILoad(idx) | FLoad(idx) | ALoad(idx) => {
                load_local::<SINGLE_SLOT>(frame, u16::from(*idx))?
            }
            LLoad(idx) | DLoad(idx) => load_local::<DUAL_SLOT>(frame, (*idx).into())?,
            ILoad0 | FLoad0 | ALoad0 => load_local::<SINGLE_SLOT>(frame, 0)?,
            ILoad1 | FLoad1 | ALoad1 => load_local::<SINGLE_SLOT>(frame, 1)?,
            ILoad2 | FLoad2 | ALoad2 => load_local::<SINGLE_SLOT>(frame, 2)?,
            ILoad3 | FLoad3 | ALoad3 => load_local::<SINGLE_SLOT>(frame, 3)?,
            LLoad0 | DLoad0 => load_local::<DUAL_SLOT>(frame, 0)?,
            LLoad1 | DLoad1 => load_local::<DUAL_SLOT>(frame, 1)?,
            LLoad2 | DLoad2 => load_local::<DUAL_SLOT>(frame, 2)?,
            LLoad3 | DLoad3 => load_local::<DUAL_SLOT>(frame, 3)?,
            IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
                let index = frame.pop_value::<SINGLE_SLOT>()?;
                let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
                let array_op = ArrayOperation::Read { array_ref, index };

                frame.push_value::<SINGLE_SLOT>(Operand::Just(def.into()))?;
                IR::Definition {
                    value: def,
                    expr: Expression::Array(array_op),
                }
            }
            LALoad | DALoad => {
                let index = frame.pop_value::<SINGLE_SLOT>()?;
                let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
                let array_op = ArrayOperation::Read { array_ref, index };

                frame.push_value::<DUAL_SLOT>(def.as_operand())?;
                IR::Definition {
                    value: def,
                    expr: Expression::Array(array_op),
                }
            }
            IStore(idx) | FStore(idx) | AStore(idx) => {
                store_local::<SINGLE_SLOT>(frame, u16::from(*idx))?
            }
            LStore(idx) | DStore(idx) => store_local::<DUAL_SLOT>(frame, u16::from(*idx))?,
            IStore0 | FStore0 | AStore0 => store_local::<SINGLE_SLOT>(frame, 0)?,
            IStore1 | FStore1 | AStore1 => store_local::<SINGLE_SLOT>(frame, 1)?,
            IStore2 | FStore2 | AStore2 => store_local::<SINGLE_SLOT>(frame, 2)?,
            IStore3 | FStore3 | AStore3 => store_local::<SINGLE_SLOT>(frame, 3)?,
            LStore0 | DStore0 => store_local::<DUAL_SLOT>(frame, 0)?,
            LStore1 | DStore1 => store_local::<DUAL_SLOT>(frame, 1)?,
            LStore2 | DStore2 => store_local::<DUAL_SLOT>(frame, 2)?,
            LStore3 | DStore3 => store_local::<DUAL_SLOT>(frame, 3)?,
            IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
                let value = frame.pop_value::<SINGLE_SLOT>()?;
                let index = frame.pop_value::<SINGLE_SLOT>()?;
                let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
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
                let value = frame.pop_value::<DUAL_SLOT>()?;
                let index = frame.pop_value::<SINGLE_SLOT>()?;
                let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
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
            Pop | Pop2 | Dup | DupX1 | DupX2 | Dup2 | Dup2X1 | Dup2X2 | Swap => {
                match jvm_instruction {
                    Pop => frame.pop()?,
                    Pop2 => frame.pop2()?,
                    Dup => frame.dup()?,
                    DupX1 => frame.dup_x1()?,
                    DupX2 => frame.dup_x2()?,
                    Dup2 => frame.dup2()?,
                    Dup2X1 => frame.dup2_x1()?,
                    Dup2X2 => frame.dup2_x2()?,
                    Swap => frame.swap()?,
                    _ => unreachable!("By outer match arm"),
                }
                IR::Nop
            }
            IAdd | FAdd => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::Add)?,
            ISub | FSub => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::Subtract)?,
            IMul | FMul => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::Multiply)?,
            IDiv | FDiv => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::Divide)?,
            IRem | FRem => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::Remainder)?,
            LDiv | DDiv => binary_op_math::<DUAL_SLOT>(frame, def, MathOperation::Divide)?,
            LAdd | DAdd => binary_op_math::<DUAL_SLOT>(frame, def, MathOperation::Add)?,
            LSub | DSub => binary_op_math::<DUAL_SLOT>(frame, def, MathOperation::Subtract)?,
            LMul | DMul => binary_op_math::<DUAL_SLOT>(frame, def, MathOperation::Multiply)?,
            LRem | DRem => binary_op_math::<DUAL_SLOT>(frame, def, MathOperation::Remainder)?,
            INeg | FNeg => {
                let value = frame.pop_value::<SINGLE_SLOT>()?;
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let math_op = MathOperation::Negate(value);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            LNeg | DNeg => {
                let operand = frame.pop_value::<DUAL_SLOT>()?;
                let value = def.as_operand();
                frame.push_value::<DUAL_SLOT>(value)?;
                let math_op = MathOperation::Negate(operand);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            IShl => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::ShiftLeft)?,
            IShr => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::ShiftRight)?,
            LShl => {
                let shift_amount = frame.pop_value::<SINGLE_SLOT>()?;
                let base = frame.pop_value::<DUAL_SLOT>()?;
                let value = def.as_operand();
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
                let value = def.as_operand();
                frame.push_value::<DUAL_SLOT>(value)?;
                let math_op = MathOperation::ShiftRight(base, shift_amount);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            LUShr => {
                let shift_amount = frame.pop_value::<SINGLE_SLOT>()?;
                let base = frame.pop_value::<DUAL_SLOT>()?;
                let value = def.as_operand();
                frame.push_value::<DUAL_SLOT>(value)?;
                let math_op = MathOperation::LogicalShiftRight(base, shift_amount);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            IUShr => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::LogicalShiftRight)?,
            IAnd => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::BitwiseAnd)?,
            IOr => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::BitwiseOr)?,
            IXor => binary_op_math::<SINGLE_SLOT>(frame, def, MathOperation::BitwiseXor)?,
            LAnd => binary_op_math::<DUAL_SLOT>(frame, def, MathOperation::BitwiseAnd)?,
            LOr => binary_op_math::<DUAL_SLOT>(frame, def, MathOperation::BitwiseOr)?,
            LXor => binary_op_math::<DUAL_SLOT>(frame, def, MathOperation::BitwiseXor)?,
            IInc(idx, constant) => {
                let idx = (*idx).into();
                let base = frame.get_local::<SINGLE_SLOT>(idx)?;
                frame.set_local::<SINGLE_SLOT>(idx, def.as_operand())?;
                let math_op = MathOperation::Increment(base, *constant);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            Wide(WideInstruction::IInc(idx, constant)) => {
                let base = frame.get_local::<SINGLE_SLOT>(*idx)?;
                frame.set_local::<SINGLE_SLOT>(*idx, def.as_operand())?;
                let math_op = MathOperation::Increment(base, *constant);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            I2F => conversion_op::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Int2Float)?,
            I2L => conversion_op::<SINGLE_SLOT, DUAL_SLOT>(frame, def, Conversion::Int2Long)?,
            I2D => conversion_op::<SINGLE_SLOT, DUAL_SLOT>(frame, def, Conversion::Int2Double)?,
            L2I => conversion_op::<DUAL_SLOT, SINGLE_SLOT>(frame, def, Conversion::Long2Int)?,
            L2F => conversion_op::<DUAL_SLOT, SINGLE_SLOT>(frame, def, Conversion::Long2Float)?,
            L2D => conversion_op::<DUAL_SLOT, DUAL_SLOT>(frame, def, Conversion::Long2Double)?,
            F2I => conversion_op::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Float2Int)?,
            F2L => conversion_op::<SINGLE_SLOT, DUAL_SLOT>(frame, def, Conversion::Float2Long)?,
            F2D => conversion_op::<SINGLE_SLOT, DUAL_SLOT>(frame, def, Conversion::Float2Double)?,
            D2I => conversion_op::<DUAL_SLOT, SINGLE_SLOT>(frame, def, Conversion::Double2Int)?,
            D2L => conversion_op::<DUAL_SLOT, DUAL_SLOT>(frame, def, Conversion::Double2Long)?,
            D2F => conversion_op::<DUAL_SLOT, SINGLE_SLOT>(frame, def, Conversion::Double2Float)?,
            I2B => conversion_op::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Int2Byte)?,
            I2C => conversion_op::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Int2Char)?,
            I2S => conversion_op::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, Conversion::Int2Short)?,
            LCmp => {
                let rhs = frame.pop_value::<DUAL_SLOT>()?;
                let lhs = frame.pop_value::<DUAL_SLOT>()?;
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let math_op = MathOperation::LongComparison(lhs, rhs);
                IR::Definition {
                    value: def,
                    expr: Expression::Math(math_op),
                }
            }
            FCmpL | FCmpG => {
                let rhs = frame.pop_value::<SINGLE_SLOT>()?;
                let lhs = frame.pop_value::<SINGLE_SLOT>()?;
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
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
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
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
            IfEq(target) => conditional_jump(frame, *target, Condition::IsZero)?,
            IfNe(target) => conditional_jump(frame, *target, Condition::IsNonZero)?,
            IfLt(target) => conditional_jump(frame, *target, Condition::IsNegative)?,
            IfGe(target) => conditional_jump(frame, *target, Condition::IsNonNegative)?,
            IfGt(target) => conditional_jump(frame, *target, Condition::IsPositive)?,
            IfLe(target) => conditional_jump(frame, *target, Condition::IsNonPositive)?,
            IfNull(target) => conditional_jump(frame, *target, Condition::IsNull)?,
            IfNonNull(target) => conditional_jump(frame, *target, Condition::IsNotNull)?,
            IfICmpEq(target) | IfACmpEq(target) => cmp_jump(frame, *target, Condition::Equal)?,
            IfICmpNe(target) | IfACmpNe(target) => cmp_jump(frame, *target, Condition::NotEqual)?,
            IfICmpGe(target) => cmp_jump(frame, *target, Condition::GreaterThanOrEqual)?,
            IfICmpLt(target) => cmp_jump(frame, *target, Condition::LessThan)?,
            IfICmpGt(target) => cmp_jump(frame, *target, Condition::GreaterThan)?,
            IfICmpLe(target) => cmp_jump(frame, *target, Condition::LessThanOrEqual)?,
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
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                IR::Definition {
                    value: def,
                    expr: value,
                }
            }
            Ret(idx) => {
                let idx = (*idx).into();
                let return_address = frame.get_local::<SINGLE_SLOT>(idx)?;
                IR::SubroutineRet(return_address)
            }
            Wide(WideInstruction::Ret(idx)) => {
                let return_address = frame.get_local::<SINGLE_SLOT>(*idx)?;
                IR::SubroutineRet(return_address)
            }
            TableSwitch {
                range,
                jump_targets,
                default,
            } => {
                let condition = frame.pop_value::<SINGLE_SLOT>()?;
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
                let condition = frame.pop_value::<SINGLE_SLOT>()?;
                IR::Switch {
                    match_value: condition,
                    default: *default,
                    branches: match_targets.clone(),
                }
            }
            IReturn | FReturn | AReturn => {
                let value = frame.pop_value::<SINGLE_SLOT>()?;
                IR::Return(Some(value))
            }
            LReturn | DReturn => {
                let value = frame.pop_value::<DUAL_SLOT>()?;
                IR::Return(Some(value))
            }
            Return => IR::Return(None),
            GetStatic(field) => {
                frame.typed_push(&field.field_type, def.as_operand())?;
                let field = field.clone();
                let field_op = FieldAccess::ReadStatic { field };
                IR::Definition {
                    value: def,
                    expr: Expression::Field(field_op),
                }
            }
            GetField(field) => {
                let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
                let field = field.clone();
                frame.typed_push(&field.field_type, def.as_operand())?;
                let field_op = FieldAccess::ReadInstance { object_ref, field };
                IR::Definition {
                    value: def,
                    expr: Expression::Field(field_op),
                }
            }
            PutStatic(field) => {
                use PrimitiveType::{Double, Long};
                let value = if let FieldType::Base(Double | Long) = field.field_type {
                    frame.pop_value::<DUAL_SLOT>()
                } else {
                    frame.pop_value::<SINGLE_SLOT>()
                }?;
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
                    frame.pop_value::<DUAL_SLOT>()
                } else {
                    frame.pop_value::<SINGLE_SLOT>()
                }?;
                let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
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
                let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
                let rhs = Expression::Call {
                    method: method_ref.clone(),
                    this: Some(object_ref),
                    args: arguments,
                };
                if let ReturnType::Some(ref return_type) = method_ref.descriptor.return_type {
                    frame.typed_push(return_type, def.as_operand())?;
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
                if let ReturnType::Some(ref return_type) = method_ref.descriptor.return_type {
                    frame.typed_push(return_type, def.as_operand())?;
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
                if let ReturnType::Some(ref return_type) = descriptor.return_type {
                    frame.typed_push(return_type, def.as_operand())?;
                }
                IR::Definition {
                    value: def,
                    expr: rhs,
                }
            }
            New(class) => {
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                IR::Definition {
                    value: def,
                    expr: Expression::New(class.clone()),
                }
            }
            ANewArray(class_ref) => {
                let count = frame.pop_value::<SINGLE_SLOT>()?;
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
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
                let count = frame.pop_value::<SINGLE_SLOT>()?;
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
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
                    .map(|_| frame.pop_value::<SINGLE_SLOT>())
                    .collect::<Result<_, _>>()?;
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let expr = Expression::Array(ArrayOperation::NewMultiDim {
                    element_type: element_type.clone(),
                    dimensions: counts,
                });
                IR::Definition { value: def, expr }
            }
            ArrayLength => {
                let array_ref = frame.pop_value::<SINGLE_SLOT>()?;
                frame.push_value::<SINGLE_SLOT>(def.as_operand())?;
                let expr = Expression::Array(ArrayOperation::Length { array_ref });
                IR::Definition { value: def, expr }
            }
            AThrow => {
                let exception_ref = frame.pop_value::<SINGLE_SLOT>()?;
                let expr = Expression::Throw(exception_ref);
                IR::Definition { value: def, expr }
            }
            CheckCast(target_type) => {
                conversion_op::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, |value| {
                    Conversion::CheckCast(value, target_type.clone())
                })?
            }
            InstanceOf(target_type) => {
                conversion_op::<SINGLE_SLOT, SINGLE_SLOT>(frame, def, |value| {
                    Conversion::InstanceOf(value, target_type.clone())
                })?
            }
            MonitorEnter => {
                let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
                let monitor_op = LockOperation::Acquire(object_ref);
                let expr = Expression::Synchronization(monitor_op);
                IR::Definition { value: def, expr }
            }
            MonitorExit => {
                let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
                let monitor_op = LockOperation::Release(object_ref);
                let expr = Expression::Synchronization(monitor_op);
                IR::Definition { value: def, expr }
            }
            Wide(
                WideInstruction::ILoad(idx)
                | WideInstruction::FLoad(idx)
                | WideInstruction::ALoad(idx),
            ) => {
                let value = frame.get_local::<SINGLE_SLOT>(*idx)?;
                frame.push_value::<SINGLE_SLOT>(value)?;
                IR::Nop
            }
            Wide(WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx)) => {
                let value = frame.get_local::<DUAL_SLOT>(*idx)?;
                frame.push_value::<DUAL_SLOT>(value)?;
                IR::Nop
            }
            Wide(
                WideInstruction::IStore(idx)
                | WideInstruction::FStore(idx)
                | WideInstruction::AStore(idx),
            ) => {
                let value = frame.pop_value::<SINGLE_SLOT>()?;
                frame.set_local::<SINGLE_SLOT>(*idx, value)?;
                IR::Nop
            }
            Wide(WideInstruction::LStore(idx) | WideInstruction::DStore(idx)) => {
                let value = frame.pop_value::<DUAL_SLOT>()?;
                frame.set_local::<DUAL_SLOT>(*idx, value)?;
                IR::Nop
            }
        };
        Ok(ir_instruction)
    }
}

#[inline]
fn load_local<const SLOT: SlotWidth>(
    frame: &mut JvmStackFrame,
    idx: u16,
) -> Result<IR, MokaIRBrewingError> {
    let value = frame.get_local::<SLOT>(idx)?;
    frame.push_value::<SLOT>(value)?;
    Ok(IR::Nop)
}

#[inline]
fn store_local<const SLOT: SlotWidth>(
    frame: &mut JvmStackFrame,
    idx: u16,
) -> Result<IR, MokaIRBrewingError> {
    let value = frame.pop_value::<SLOT>()?;
    frame.set_local::<SLOT>(idx, value)?;
    Ok(IR::Nop)
}

#[inline]
fn conditional_jump(
    frame: &mut JvmStackFrame,
    target: ProgramCounter,
    condition: impl FnOnce(Operand) -> Condition,
) -> Result<IR, MokaIRBrewingError> {
    let operand = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(IR::Jump {
        condition: Some(condition(operand)),
        target,
    })
}

#[inline]
fn cmp_jump(
    frame: &mut JvmStackFrame,
    target: ProgramCounter,
    condition: impl FnOnce(Operand, Operand) -> Condition,
) -> Result<IR, MokaIRBrewingError> {
    let rhs = frame.pop_value::<SINGLE_SLOT>()?;
    let lhs = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(IR::Jump {
        condition: Some(condition(lhs, rhs)),
        target,
    })
}

#[inline]
fn conversion_op<const OPERAND_SLOT: SlotWidth, const RESULT_SLOT: SlotWidth>(
    frame: &mut JvmStackFrame,
    def: LocalValue,
    conversion: impl FnOnce(Operand) -> Conversion,
) -> Result<IR, MokaIRBrewingError> {
    let operand = frame.pop_value::<OPERAND_SLOT>()?;
    frame.push_value::<RESULT_SLOT>(def.as_operand())?;
    Ok(IR::Definition {
        value: def,
        expr: Expression::Conversion(conversion(operand)),
    })
}

#[inline]
fn binary_op_math<const SLOT: SlotWidth>(
    frame: &mut JvmStackFrame,
    def_id: LocalValue,
    math: impl FnOnce(Operand, Operand) -> MathOperation,
) -> Result<IR, MokaIRBrewingError> {
    let rhs = frame.pop_value::<SLOT>()?;
    let lhs = frame.pop_value::<SLOT>()?;
    let value = def_id.as_operand();
    frame.push_value::<SLOT>(value)?;

    let expr = Expression::Math(math(lhs, rhs));
    Ok(IR::Definition {
        value: def_id,
        expr,
    })
}
