use std::iter::once;

use crate::{
    analysis::stack_frame::{ir::MokaInstruction, Expression, FrameValue, Identifier},
    elements::{
        instruction::{Instruction, ProgramCounter},
        ConstantValue, ReturnType,
    },
    types::{FieldType, PrimitiveType},
};

use super::{StackFrame, StackFrameAnalyzer, StackFrameError};

impl StackFrameAnalyzer {
    pub(super) fn run_instruction(
        &mut self,
        insn: &Instruction,
        pc: ProgramCounter,
        frame: &mut StackFrame,
    ) -> Result<(), StackFrameError> {
        use Instruction::*;
        // TODO: Clear preceding kept instructions if the current instruction should be kept
        match insn {
            Nop => {}
            AConstNull => {
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(ConstantValue::Null),
                    },
                );
                frame.push_value(def_id.into());
            }
            IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
                let def_id = Identifier::Val(pc.into());
                let value = (insn.opcode() as i32) - 3;
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(ConstantValue::Integer(value)),
                    },
                );
                frame.push_value(def_id.into());
            }
            LConst0 | LConst1 => {
                let def_id = Identifier::Val(pc.into());
                let value = (insn.opcode() as i64) - 9;
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(ConstantValue::Long(value)),
                    },
                );
                frame.push_value(def_id.into());
            }
            FConst0 | FConst1 | FConst2 => {
                let def_id = Identifier::Val(pc.into());
                let value = (insn.opcode() as f32) - 11.0;
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(ConstantValue::Float(value)),
                    },
                );
                frame.push_value(def_id.into());
            }
            DConst0 | DConst1 => {
                let def_id = Identifier::Val(pc.into());
                let value = (insn.opcode() as f64) - 14.0;
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(ConstantValue::Double(value)),
                    },
                );
                frame.push_value(def_id.into());
            }
            BiPush(value) => {
                let def_id = Identifier::Val(pc.into());
                let value = *value as i32;
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(ConstantValue::Integer(value)),
                    },
                );
                frame.push_value(def_id.into());
            }
            SiPush(value) => {
                let def_id = Identifier::Val(pc.into());
                let value = *value as i32;
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(ConstantValue::Integer(value)),
                    },
                );
                frame.push_value(def_id.into());
            }
            Ldc(value) | LdcW(value) => {
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(value.clone()),
                    },
                );
                frame.push_value(def_id.into());
            }
            Ldc2W(value) => {
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Const(value.clone()),
                    },
                );
                frame.push_value(def_id.into());
                frame.push_padding();
            }
            ILoad(idx) | FLoad(idx) | ALoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value);
            }
            LLoad(idx) | DLoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value);
                frame.push_padding();
            }
            ILoad0 | FLoad0 | ALoad0 => {
                let value = frame.get_local(0usize)?;
                frame.push_value(value);
            }
            ILoad1 | FLoad1 | ALoad1 => {
                let value = frame.get_local(1usize)?;
                frame.push_value(value);
            }
            ILoad2 | FLoad2 | ALoad2 => {
                let value = frame.get_local(2usize)?;
                frame.push_value(value);
            }
            ILoad3 | FLoad3 | ALoad3 => {
                let value = frame.get_local(3usize)?;
                frame.push_value(value);
            }
            LLoad0 | DLoad0 => {
                let value = frame.get_local(0usize)?;
                frame.push_value(value);
                frame.push_padding();
            }
            LLoad1 | DLoad1 => {
                let value = frame.get_local(1usize)?;
                frame.push_value(value);
                frame.push_padding();
            }
            LLoad2 | DLoad2 => {
                let value = frame.get_local(2usize)?;
                frame.push_value(value);
                frame.push_padding();
            }
            LLoad3 | DLoad3 => {
                let value = frame.get_local(3usize)?;
                frame.push_value(value);
                frame.push_padding();
            }
            IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
                let index = frame.pop_value()?;
                let arrayref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![index, arrayref],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            LALoad | DALoad => {
                let index = frame.pop_value()?;
                let arrayref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![index, arrayref],
                        },
                    },
                );
                frame.push_value(def_id.into());
                frame.push_padding();
            }
            IStore(idx) | FStore(idx) | AStore(idx) => {
                let value = frame.pop_value()?;
                frame.set_local(*idx, value);
            }
            LStore(idx) | DStore(idx) => {
                let value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                frame.set_local(*idx, value);
                frame.set_local_padding(*idx + 1);
            }
            IStore0 | FStore0 | AStore0 => {
                let value = frame.pop_value()?;
                frame.set_local(0usize, value);
            }
            IStore1 | FStore1 | AStore1 => {
                let value = frame.pop_value()?;
                frame.set_local(1usize, value);
            }
            IStore2 | FStore2 | AStore2 => {
                let value = frame.pop_value()?;
                frame.set_local(2usize, value);
            }
            IStore3 | FStore3 | AStore3 => {
                let value = frame.pop_value()?;
                frame.set_local(3usize, value);
            }
            LStore0 | DStore0 => {
                let value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                frame.set_local(0usize, value);
                frame.set_local_padding(1usize as usize);
            }
            LStore1 | DStore1 => {
                let value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                frame.set_local(1usize, value);
                frame.set_local_padding(2usize);
            }
            LStore2 | DStore2 => {
                let value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                frame.set_local(2usize, value);
                frame.set_local_padding(2usize);
            }
            LStore3 | DStore3 => {
                let value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                frame.set_local(3usize, value);
                frame.set_local_padding(4usize);
            }
            IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
                let value = frame.pop_value()?;
                let index = frame.pop_value()?;
                let arrayref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![index, arrayref, value],
                        },
                    },
                );
            }
            LAStore | DAStore => {
                let _value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                let index = frame.pop_value()?;
                let arrayref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![index, arrayref, value],
                        },
                    },
                );
            }
            Pop => {
                frame.pop_value()?;
            }
            Pop2 => {
                frame.pop_value()?;
                frame.pop_value()?;
            }
            Dup => {
                let value = frame.pop_value()?;
                frame.push_value(value.clone());
                frame.push_value(value);
            }
            DupX1 => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                frame.push_value(value1.clone());
                frame.push_value(value2);
                frame.push_value(value1);
            }
            DupX2 => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                let value3 = frame.pop_value()?;
                frame.push_value(value1.clone());
                frame.push_value(value3);
                frame.push_value(value2);
                frame.push_value(value1);
            }
            Dup2 => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                frame.push_value(value2.clone());
                frame.push_value(value1.clone());
                frame.push_value(value2);
                frame.push_value(value1);
            }
            Dup2X1 => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                let value3 = frame.pop_value()?;
                frame.push_value(value2.clone());
                frame.push_value(value1.clone());
                frame.push_value(value3);
                frame.push_value(value2);
                frame.push_value(value1);
            }
            Dup2X2 => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                let value3 = frame.pop_value()?;
                let value4 = frame.pop_value()?;
                frame.push_value(value2.clone());
                frame.push_value(value1.clone());
                frame.push_value(value4);
                frame.push_value(value3);
                frame.push_value(value2);
                frame.push_value(value1);
            }
            Swap => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                frame.push_value(value1);
                frame.push_value(value2);
            }
            IAdd | FAdd | ISub | FSub | IMul | FMul | IDiv | FDiv | IRem | FRem => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value2, value1],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            LAdd | DAdd | LSub | DSub | LMul | DMul | LDiv | DDiv | LRem | DRem => {
                let value1_padding = frame.pop_value()?;
                let value1 = frame.pop_value()?;
                let value2_padding = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value2, value1, value2_padding, value1_padding],
                        },
                    },
                );
                frame.push_value(def_id.into());
                frame.push_padding();
            }
            INeg | FNeg => {
                let value = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            LNeg | DNeg => {
                let value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value, value_padding],
                        },
                    },
                );
                frame.push_value(def_id.into());
                frame.push_padding();
            }
            IShl | LShl | IShr | LShr | IUShr | LUShr | IAnd | LAnd | IOr | LOr | IXor | LXor => {
                let value1 = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value2, value1],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            IInc(idx, _) => {
                let base = frame.get_local(*idx)?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![base],
                        },
                    },
                );
                frame.set_local(*idx, def_id.into());
            }
            WideIInc(idx, _) => {
                let base = frame.get_local(*idx)?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![base],
                        },
                    },
                );
                frame.set_local(*idx, def_id.into());
            }
            I2F | I2B | I2C | I2S | F2I => {
                let value = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            I2L | I2D | F2L | F2D => {
                let value = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value],
                        },
                    },
                );
                frame.push_value(def_id.into());
                frame.push_padding();
            }
            L2I | L2F | D2I | D2F => {
                let _padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            L2D | D2L => {
                let _value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value],
                        },
                    },
                );
                frame.push_value(def_id.into());
                frame.push_padding();
            }
            LCmp | FCmpL | FCmpG | DCmpL | DCmpG => {
                let _value1_padding = frame.pop_value()?;
                let value1 = frame.pop_value()?;
                let _value2_padding = frame.pop_value()?;
                let value2 = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value1, value2],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            IfEq(target) | IfNe(target) | IfLt(target) | IfGe(target) | IfGt(target)
            | IfLe(target) | IfNull(target) | IfNonNull(target) | IfICmpEq(target)
            | IfICmpNe(target) | IfICmpLt(target) | IfICmpGe(target) | IfICmpGt(target)
            | IfICmpLe(target) | IfACmpEq(target) | IfACmpNe(target) => {
                let value = frame.pop_value()?;
                self.code_map.insert(
                    pc,
                    MokaInstruction::ConditionalJump {
                        condition: value,
                        target: *target,
                        instruction: insn.clone(),
                    },
                );
            }
            Goto(target) | GotoW(target) => {
                self.code_map
                    .insert(pc, MokaInstruction::Jump { target: *target });
            }
            Jsr(_) | JsrW(_) => {
                let value = Expression::ReturnAddress(pc);
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: value,
                    },
                );
                frame.push_value(def_id.into());
            }
            Ret(idx) => {
                let return_address = frame.get_local(*idx)?;
            }
            WideRet(idx) => {
                let return_address = frame.get_local(*idx)?;
            }
            TableSwitch { .. } | LookupSwitch { .. } => {
                let condition = frame.pop_value()?;
                self.code_map.insert(
                    pc,
                    MokaInstruction::Switch {
                        condition,
                        instruction: insn.clone(),
                    },
                );
            }
            IReturn | FReturn | AReturn => {
                let value = frame.pop_value()?;
                self.code_map
                    .insert(pc, MokaInstruction::Return { value: Some(value) });
            }
            LReturn | DReturn => {
                frame.pop_padding()?;
                let value = frame.pop_value()?;
                self.code_map
                    .insert(pc, MokaInstruction::Return { value: Some(value) });
            }
            Return => {
                self.code_map
                    .insert(pc, MokaInstruction::Return { value: None });
            }
            GetStatic(field) => {
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![],
                        },
                    },
                );
                frame.push_value(def_id.into());
                match field.field_type {
                    FieldType::Base(PrimitiveType::Long)
                    | FieldType::Base(PrimitiveType::Double) => {
                        frame.push_padding();
                    }
                    _ => {}
                }
            }
            GetField(field) => {
                let objectref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![objectref],
                        },
                    },
                );
                frame.push_value(def_id.into());
                match field.field_type {
                    FieldType::Base(PrimitiveType::Long)
                    | FieldType::Base(PrimitiveType::Double) => {
                        frame.push_padding();
                    }
                    _ => {}
                }
            }
            PutStatic(field) => {
                match field.field_type {
                    FieldType::Base(PrimitiveType::Long)
                    | FieldType::Base(PrimitiveType::Double) => {
                        frame.pop_value()?;
                    }
                    _ => {}
                }
                let value = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![value],
                        },
                    },
                );
            }
            PutField(field) => {
                match field.field_type {
                    FieldType::Base(PrimitiveType::Long)
                    | FieldType::Base(PrimitiveType::Double) => {
                        frame.pop_value()?;
                        frame.pop_value()?;
                    }
                    _ => {
                        frame.pop_value()?;
                    }
                }
                let value = frame.pop_value()?;
                let objectref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![objectref, value],
                        },
                    },
                );
            }
            InvokeVirtual(method_ref) | InvokeSpecial(method_ref) => {
                let arguments: Vec<_> = method_ref
                    .descriptor()
                    .parameters_types
                    .iter()
                    .map(|_| frame.pop_value())
                    .collect::<Result<_, _>>()?;
                let objectref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![objectref]
                                .into_iter()
                                .chain(arguments.into_iter().rev())
                                .collect(),
                        },
                    },
                );
                frame.push_value(def_id.into());
                match method_ref.descriptor().return_type {
                    ReturnType::Some(FieldType::Base(PrimitiveType::Long))
                    | ReturnType::Some(FieldType::Base(PrimitiveType::Double)) => {
                        frame.push_padding();
                    }
                    _ => {}
                }
            }
            InvokeInterface(i_method_ref, _) => {
                let arguments: Vec<_> = i_method_ref
                    .descriptor
                    .parameters_types
                    .iter()
                    .map(|_| frame.pop_value())
                    .collect::<Result<_, _>>()?;
                let objectref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: once(objectref).chain(arguments.into_iter().rev()).collect(),
                        },
                    },
                );
                frame.push_value(def_id.into());
                match i_method_ref.descriptor.return_type {
                    ReturnType::Some(FieldType::Base(PrimitiveType::Long))
                    | ReturnType::Some(FieldType::Base(PrimitiveType::Double)) => {
                        frame.push_padding();
                    }
                    _ => {}
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
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments,
                        },
                    },
                );
                frame.push_value(def_id.into());
                match method_ref.descriptor().return_type {
                    ReturnType::Some(FieldType::Base(PrimitiveType::Long))
                    | ReturnType::Some(FieldType::Base(PrimitiveType::Double)) => {
                        frame.push_padding();
                    }
                    _ => {}
                }
            }
            InvokeDynamic { descriptor, .. } => {
                let arguments: Vec<_> = descriptor
                    .parameters_types
                    .iter()
                    .map(|_| frame.pop_value())
                    .rev()
                    .collect::<Result<_, _>>()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments,
                        },
                    },
                );
                frame.push_value(def_id.into());
                match descriptor.return_type {
                    ReturnType::Some(FieldType::Base(PrimitiveType::Long))
                    | ReturnType::Some(FieldType::Base(PrimitiveType::Double)) => {
                        frame.push_padding();
                    }
                    _ => {}
                }
            }
            New(_) | NewArray(_) | ANewArray(_) | MultiANewArray(_, _) => {
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            ArrayLength => {
                let arrayref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![arrayref],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            AThrow => {
                let _objectref = frame.pop_value()?;
            }
            CheckCast(_) | InstanceOf(_) => {
                let objectref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![objectref],
                        },
                    },
                );
                frame.push_value(def_id.into());
            }
            MonitorEnter | MonitorExit => {
                let objectref = frame.pop_value()?;
                let def_id = Identifier::Val(pc.into());
                self.code_map.insert(
                    pc,
                    MokaInstruction::Assignment {
                        lhs: def_id,
                        rhs: Expression::Expr {
                            instruction: insn.clone(),
                            arguments: vec![objectref],
                        },
                    },
                );
            }
            WideILoad(idx) | WideFLoad(idx) | WideALoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value);
            }
            WideLLoad(idx) | WideDLoad(idx) => {
                let value = frame.get_local(*idx)?;
                frame.push_value(value);
                frame.push_padding();
            }
            WideIStore(idx) | WideFStore(idx) | WideAStore(idx) => {
                let value = frame.pop_value()?;
                frame.set_local(*idx, value);
            }
            WideLStore(idx) | WideDStore(idx) => {
                let value_padding = frame.pop_value()?;
                let value = frame.pop_value()?;
                frame.set_local(*idx, value);
                frame.set_local_padding(idx + 1);
            }
            Breakpoint | ImpDep1 | ImpDep2 => unimplemented!("These op codes are reserved"),
        };

        Ok(())
    }
}
