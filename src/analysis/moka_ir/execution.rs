use std::{f32::consts::E, iter::once};

use crate::{
    analysis::moka_ir::{
        moka_instruction::{Expression, Identifier, MokaInstruction as IR},
        ArrayOperation, FieldAccess,
    },
    elements::{
        instruction::{Instruction, ProgramCounter, TypeReference},
        ConstantValue, ReturnType,
    },
    types::{FieldType, PrimitiveType},
};

use super::{MokaIRGenerationError, MokaIRGenerator, StackFrame};

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
            IAdd | FAdd | ISub | FSub | IMul | FMul | IDiv | FDiv | IRem | FRem => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value2, value1],
                    },
                }
            }
            LAdd | DAdd | LSub | DSub | LMul | DMul | LDiv | DDiv | LRem | DRem => {
                let value1_padding = frame.pop_value()?;
                let value1 = frame.pop_value()?;
                let value2_padding = frame.pop_value()?;
                let value2 = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                frame.push_padding()?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value2, value1, value2_padding, value1_padding],
                    },
                }
            }
            INeg | FNeg => {
                let value = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                }
            }
            LNeg | DNeg => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                frame.push_padding()?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                }
            }
            IShl | LShl | IShr | LShr | IUShr | LUShr | IAnd | LAnd | IOr | LOr | IXor | LXor => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value2, value1],
                    },
                }
            }
            IInc(idx, _) => {
                let base = frame.get_local(*idx)?;

                frame.set_local(*idx, def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![base],
                    },
                }
            }
            WideIInc(idx, _) => {
                let base = frame.get_local(*idx)?;

                frame.set_local(*idx, def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![base],
                    },
                }
            }
            I2F | I2B | I2C | I2S | F2I => {
                let value = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                }
            }
            I2L | I2D | F2L | F2D => {
                let value = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                frame.push_padding()?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                }
            }
            L2I | L2F | D2I | D2F => {
                let _padding = frame.pop_value()?;
                let value = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                }
            }
            L2D | D2L => {
                let _value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                frame.push_padding()?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                }
            }
            LCmp | FCmpL | FCmpG | DCmpL | DCmpG => {
                frame.pop_padding()?;
                let value1 = frame.pop_value()?;
                frame.pop_padding()?;
                let value2 = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![value1, value2],
                    },
                }
            }
            IfEq(target) | IfNe(target) | IfLt(target) | IfGe(target) | IfGt(target)
            | IfLe(target) | IfNull(target) | IfNonNull(target) => {
                let value = frame.pop_value()?;
                IR::UnitaryConditionalJump {
                    condition: value,
                    target: *target,
                    instruction: insn.clone(),
                }
            }
            IfICmpEq(target) | IfICmpNe(target) | IfICmpLt(target) | IfICmpGe(target)
            | IfICmpGt(target) | IfICmpLe(target) | IfACmpEq(target) | IfACmpNe(target) => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                IR::BinaryConditionalJump {
                    condition: [value1, value2],
                    target: *target,
                    instruction: insn.clone(),
                }
            }
            Goto(target) | GotoW(target) => IR::Jump { target: *target },
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
                    condition,
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
                let array_op = ArrayOperation::NewMD {
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
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![exception_ref],
                    },
                }
            }
            CheckCast(_) | InstanceOf(_) => {
                let object_ref = frame.pop_value()?;

                frame.push_value(def_id.into())?;
                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![object_ref],
                    },
                }
            }
            MonitorEnter | MonitorExit => {
                let object_ref = frame.pop_value()?;

                IR::Assignment {
                    lhs: def_id,
                    rhs: Expression::Insn {
                        instruction: insn.clone(),
                        arguments: vec![object_ref],
                    },
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
}
