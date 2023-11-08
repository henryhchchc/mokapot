use crate::{
    analysis::stack_frame::{DefId, FrameValue, JavaAbstractValue},
    elements::{
        instruction::{Instruction, ProgramCounter},
        ConstantValue, ReturnType,
    },
    types::{FieldType, PrimitiveType},
};

use super::{StackFrame, StackFrameAnalyzer};

impl StackFrameAnalyzer {
    pub(super) fn run_instruction(
        &mut self,
        insn: &Instruction,
        pc: ProgramCounter,
        frame: &mut StackFrame,
    ) {
        use Instruction::*;
        match insn {
            Nop => {}
            AConstNull => {
                let def_id = DefId::At(pc);
                self.defs
                    .insert(def_id, JavaAbstractValue::Const(ConstantValue::Null));
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 => {
                let def_id = DefId::At(pc);
                let value = (insn.opcode() as i32) - 3;
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Const(ConstantValue::Integer(value)),
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            LConst0 | LConst1 => {
                let def_id = DefId::At(pc);
                let value = (insn.opcode() as i64) - 9;
                self.defs
                    .insert(def_id, JavaAbstractValue::Const(ConstantValue::Long(value)));
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            FConst0 | FConst1 | FConst2 => {
                let def_id = DefId::At(pc);
                let value = (insn.opcode() as f32) - 11.0;
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Const(ConstantValue::Float(value)),
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            DConst0 | DConst1 => {
                let def_id = DefId::At(pc);
                let value = (insn.opcode() as f64) - 14.0;
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Const(ConstantValue::Double(value)),
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            BiPush(value) => {
                let def_id = DefId::At(pc);
                let value = *value as i32;
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Const(ConstantValue::Integer(value)),
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            SiPush(value) => {
                let def_id = DefId::At(pc);
                let value = *value as i32;
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Const(ConstantValue::Integer(value)),
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            Ldc(value) | LdcW(value) => {
                let def_id = DefId::At(pc);
                self.defs
                    .insert(def_id, JavaAbstractValue::Const(value.clone()));
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            Ldc2W(value) => {
                let def_id = DefId::At(pc);
                self.defs
                    .insert(def_id, JavaAbstractValue::Const(value.clone()));
                frame.operand_stack.push(FrameValue::Def(def_id));
                frame.operand_stack.push(FrameValue::Padding);
            }
            ILoad(idx) | FLoad(idx) | ALoad(idx) => {
                let value = frame.local_variables[*idx as usize]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
            }
            LLoad(idx) | DLoad(idx) => {
                let value = frame.local_variables[*idx as usize]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
                frame.operand_stack.push(FrameValue::Padding);
            }
            ILoad0 | FLoad0 | ALoad0 => {
                let value = frame.local_variables[0]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
            }
            ILoad1 | FLoad1 | ALoad1 => {
                let value = frame.local_variables[1]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
            }
            ILoad2 | FLoad2 | ALoad2 => {
                let value = frame.local_variables[2]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
            }
            ILoad3 | FLoad3 | ALoad3 => {
                let value = frame.local_variables[3]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
            }
            LLoad0 | DLoad0 => {
                let value = frame.local_variables[0]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
                frame.operand_stack.push(FrameValue::Padding);
            }
            LLoad1 | DLoad1 => {
                let value = frame.local_variables[1]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
                frame.operand_stack.push(FrameValue::Padding);
            }
            LLoad2 | DLoad2 => {
                let value = frame.local_variables[2]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
                frame.operand_stack.push(FrameValue::Padding);
            }
            LLoad3 | DLoad3 => {
                let value = frame.local_variables[3]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
                frame.operand_stack.push(FrameValue::Padding);
            }
            IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
                let index = frame.operand_stack.pop().expect("Fail to pop stack");
                let arrayref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![index, arrayref],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            LALoad | DALoad => {
                let index = frame.operand_stack.pop().expect("Fail to pop stack");
                let arrayref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![index, arrayref],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                frame.operand_stack.push(FrameValue::Padding);
            }
            IStore(idx) | FStore(idx) | AStore(idx) => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[*idx as usize].replace(value);
            }
            LStore(idx) | DStore(idx) => {
                let value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[*idx as usize].replace(value);
                frame.local_variables[*idx as usize + 1].replace(value_padding);
            }
            IStore0 | FStore0 | AStore0 => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[0].replace(value);
            }
            IStore1 | FStore1 | AStore1 => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[1].replace(value);
            }
            IStore2 | FStore2 | AStore2 => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[2].replace(value);
            }
            IStore3 | FStore3 | AStore3 => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[3].replace(value);
            }
            LStore0 | DStore0 => {
                let value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[0].replace(value);
                frame.local_variables[1].replace(value_padding);
            }
            LStore1 | DStore1 => {
                let value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[1].replace(value);
                frame.local_variables[2].replace(value_padding);
            }
            LStore2 | DStore2 => {
                let value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[2].replace(value);
                frame.local_variables[3].replace(value_padding);
            }
            LStore3 | DStore3 => {
                let value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[3].replace(value);
                frame.local_variables[4].replace(value_padding);
            }
            IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                let index = frame.operand_stack.pop().expect("Fail to pop stack");
                let arrayref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![index, arrayref, value],
                    },
                );
            }
            LAStore | DAStore => {
                let _value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                let index = frame.operand_stack.pop().expect("Fail to pop stack");
                let arrayref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![index, arrayref, value],
                    },
                );
            }
            Pop => {
                frame.operand_stack.pop().expect("Fail to pop stack");
            }
            Pop2 => {
                frame.operand_stack.pop().expect("Fail to pop stack");
                frame.operand_stack.pop().expect("Fail to pop stack");
            }
            Dup => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.operand_stack.push(value.clone());
                frame.operand_stack.push(value);
            }
            DupX1 => {
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.operand_stack.push(value1.clone());
                frame.operand_stack.push(value2);
                frame.operand_stack.push(value1);
            }
            DupX2 => {
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value3 = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.operand_stack.push(value1.clone());
                frame.operand_stack.push(value3);
                frame.operand_stack.push(value2);
                frame.operand_stack.push(value1);
            }
            Dup2 => {
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.operand_stack.push(value2.clone());
                frame.operand_stack.push(value1.clone());
                frame.operand_stack.push(value2);
                frame.operand_stack.push(value1);
            }
            Dup2X1 => {
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value3 = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.operand_stack.push(value2.clone());
                frame.operand_stack.push(value1.clone());
                frame.operand_stack.push(value3);
                frame.operand_stack.push(value2);
                frame.operand_stack.push(value1);
            }
            Dup2X2 => {
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value3 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value4 = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.operand_stack.push(value2.clone());
                frame.operand_stack.push(value1.clone());
                frame.operand_stack.push(value4);
                frame.operand_stack.push(value3);
                frame.operand_stack.push(value2);
                frame.operand_stack.push(value1);
            }
            Swap => {
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.operand_stack.push(value1);
                frame.operand_stack.push(value2);
            }
            IAdd | FAdd | ISub | FSub | IMul | FMul | IDiv | FDiv | IRem | FRem => {
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value2, value1],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            LAdd | DAdd | LSub | DSub | LMul | DMul | LDiv | DDiv | LRem | DRem => {
                let value1_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value2, value1, value2_padding, value1_padding],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                frame.operand_stack.push(FrameValue::Padding);
            }
            INeg | FNeg => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            LNeg | DNeg => {
                let value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value, value_padding],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                frame.operand_stack.push(FrameValue::Padding);
            }
            IShl | LShl | IShr | LShr | IUShr | LUShr | IAnd | LAnd | IOr | LOr | IXor | LXor => {
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value2, value1],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            IInc(idx, _) => {
                let base = frame.local_variables[*idx as usize]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![base],
                    },
                );
                frame.local_variables[*idx as usize].replace(FrameValue::Def(def_id));
            }
            WideIInc(idx, _) => {
                let base = frame.local_variables[*idx as usize]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![base],
                    },
                );
                frame.local_variables[*idx as usize].replace(FrameValue::Def(def_id));
            }
            I2F | I2B | I2C | I2S | F2I => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            I2L | I2D | F2L | F2D => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                frame.operand_stack.push(FrameValue::Padding);
            }
            L2I | L2F | D2I | D2F => {
                let _value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            L2D | D2L => {
                let _value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                frame.operand_stack.push(FrameValue::Padding);
            }
            LCmp | FCmpL | FCmpG | DCmpL | DCmpG => {
                let _value1_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value1 = frame.operand_stack.pop().expect("Fail to pop stack");
                let _value2_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value2 = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![value1, value2],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            IfEq(_) | IfNe(_) | IfLt(_) | IfGe(_) | IfGt(_) | IfLe(_) | IfNull(_)
            | IfNonNull(_) | IfICmpEq(_) | IfICmpNe(_) | IfICmpLt(_) | IfICmpGe(_)
            | IfICmpGt(_) | IfICmpLe(_) | IfACmpEq(_) | IfACmpNe(_) => {
                let _value = frame.operand_stack.pop().expect("Fail to pop stack");
            }
            Goto(_) | GotoW(_) => {}
            Jsr(_) | JsrW(_) => {
                let value = JavaAbstractValue::ReturnAddress(pc);
                let def_id = DefId::At(pc);
                self.defs.insert(def_id, value);
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            Ret(idx) => {
                let _return_address = frame.local_variables[*idx as usize]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
            }
            WideRet(idx) => {
                let _return_address = frame.local_variables[*idx as usize]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
            }
            TableSwitch { .. } | LookupSwitch { .. } => {
                let _key = frame.operand_stack.pop().expect("Fail to pop stack");
            }
            IReturn | FReturn | AReturn => {
                let _value = frame.operand_stack.pop().expect("Fail to pop stack");
            }
            LReturn | DReturn => {
                let _value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let _value = frame.operand_stack.pop().expect("Fail to pop stack");
            }
            Return => {}
            GetStatic(field) => {
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                match field.field_type {
                    FieldType::Base(PrimitiveType::Long)
                    | FieldType::Base(PrimitiveType::Double) => {
                        frame.operand_stack.push(FrameValue::Padding);
                    }
                    _ => {}
                }
            }
            GetField(field) => {
                let objectref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![objectref],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                match field.field_type {
                    FieldType::Base(PrimitiveType::Long)
                    | FieldType::Base(PrimitiveType::Double) => {
                        frame.operand_stack.push(FrameValue::Padding);
                    }
                    _ => {}
                }
            }
            PutStatic(field) => {
                match field.field_type {
                    FieldType::Base(PrimitiveType::Long)
                    | FieldType::Base(PrimitiveType::Double) => {
                        frame.operand_stack.pop().expect("Fail to pop stack");
                    }
                    _ => {}
                }
                let _value = frame.operand_stack.pop().expect("Fail to pop stack");
            }
            PutField(field) => {
                match field.field_type {
                    FieldType::Base(PrimitiveType::Long)
                    | FieldType::Base(PrimitiveType::Double) => {
                        frame.operand_stack.pop().expect("Fail to pop stack");
                        frame.operand_stack.pop().expect("Fail to pop stack");
                    }
                    _ => {
                        frame.operand_stack.pop().expect("Fail to pop stack");
                    }
                }
                let _value = frame.operand_stack.pop().expect("Fail to pop stack");
                let _objectref = frame.operand_stack.pop().expect("Fail to pop stack");
            }
            InvokeVirtual(method_ref) | InvokeSpecial(method_ref) => {
                let arguments = method_ref
                    .descriptor()
                    .parameters_types
                    .iter()
                    .map(|_| frame.operand_stack.pop().expect("Fail to pop stack"))
                    .collect::<Vec<_>>();
                let objectref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: std::iter::once(objectref)
                            .chain(arguments.into_iter().rev())
                            .collect(),
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                match method_ref.descriptor().return_type {
                    ReturnType::Some(FieldType::Base(PrimitiveType::Long))
                    | ReturnType::Some(FieldType::Base(PrimitiveType::Double)) => {
                        frame.operand_stack.push(FrameValue::Padding);
                    }
                    _ => {}
                }
            }
            InvokeInterface(i_method_ref, _) => {
                let arguments = i_method_ref
                    .descriptor
                    .parameters_types
                    .iter()
                    .map(|_| frame.operand_stack.pop().expect("Fail to pop stack"))
                    .collect::<Vec<_>>();
                let objectref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: std::iter::once(objectref)
                            .chain(arguments.into_iter().rev())
                            .collect(),
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                match i_method_ref.descriptor.return_type {
                    ReturnType::Some(FieldType::Base(PrimitiveType::Long))
                    | ReturnType::Some(FieldType::Base(PrimitiveType::Double)) => {
                        frame.operand_stack.push(FrameValue::Padding);
                    }
                    _ => {}
                }
            }
            InvokeStatic(method_ref) => {
                let mut arguments = method_ref
                    .descriptor()
                    .parameters_types
                    .iter()
                    .map(|_| frame.operand_stack.pop().expect("Fail to pop stack"))
                    .collect::<Vec<_>>();
                arguments.reverse();
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments,
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                match method_ref.descriptor().return_type {
                    ReturnType::Some(FieldType::Base(PrimitiveType::Long))
                    | ReturnType::Some(FieldType::Base(PrimitiveType::Double)) => {
                        frame.operand_stack.push(FrameValue::Padding);
                    }
                    _ => {}
                }
            }
            InvokeDynamic { descriptor, .. } => {
                let arguments = descriptor
                    .parameters_types
                    .iter()
                    .map(|_| frame.operand_stack.pop().expect("Fail to pop stack"))
                    .rev()
                    .collect::<Vec<_>>();
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments,
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
                match descriptor.return_type {
                    ReturnType::Some(FieldType::Base(PrimitiveType::Long))
                    | ReturnType::Some(FieldType::Base(PrimitiveType::Double)) => {
                        frame.operand_stack.push(FrameValue::Padding);
                    }
                    _ => {}
                }
            }
            New(_) | NewArray(_) | ANewArray(_) | MultiANewArray(_, _) => {
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            ArrayLength => {
                let arrayref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![arrayref],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            AThrow => {
                let _objectref = frame.operand_stack.pop().expect("Fail to pop stack");
            }
            CheckCast(_) | InstanceOf(_) => {
                let objectref = frame.operand_stack.pop().expect("Fail to pop stack");
                let def_id = DefId::At(pc);
                self.defs.insert(
                    def_id,
                    JavaAbstractValue::Expression {
                        instruction: insn.clone(),
                        arguments: vec![objectref],
                    },
                );
                frame.operand_stack.push(FrameValue::Def(def_id));
            }
            MonitorEnter | MonitorExit => {
                let _objectref = frame.operand_stack.pop().expect("Fail to pop stack");
            }
            WideILoad(idx) | WideFLoad(idx) | WideALoad(idx) => {
                let value = frame.local_variables[*idx as usize]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
            }
            WideLLoad(idx) | WideDLoad(idx) => {
                let value = frame.local_variables[*idx as usize]
                    .as_ref()
                    .expect("Fail to get local")
                    .clone();
                frame.operand_stack.push(value);
                frame.operand_stack.push(FrameValue::Padding);
            }
            WideIStore(idx) | WideFStore(idx) | WideAStore(idx) => {
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[*idx as usize].replace(value);
            }
            WideLStore(idx) | WideDStore(idx) => {
                let value_padding = frame.operand_stack.pop().expect("Fail to pop stack");
                let value = frame.operand_stack.pop().expect("Fail to pop stack");
                frame.local_variables[*idx as usize].replace(value);
                frame.local_variables[*idx as usize + 1].replace(value_padding);
            }
            Breakpoint | ImpDep1 | ImpDep2 => unimplemented!("These op codes are reserved"),
        };
    }
}
