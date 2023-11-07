use std::collections::HashMap;

use crate::{
    elements::{
        annotation::TypeAnnotation,
        parsing::code::{LocalVariableDescAttr, LocalVariableTypeAttr},
        references::ClassReference,
    },
    types::FieldType,
};

use super::{Instruction, ProgramCounter};

#[derive(Debug)]
pub struct MethodBody {
    pub max_stack: u16,
    pub max_locals: u16,
    pub instructions: Vec<(ProgramCounter, Instruction)>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub line_number_table: Option<Vec<LineNumberTableEntry>>,
    pub local_variable_table: Option<LocalVariableTable>,
    pub stack_map_table: Option<Vec<StackMapFrame>>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
}

impl MethodBody {
    pub fn instruction_at(&self, pc: ProgramCounter) -> Option<&Instruction> {
        self.instructions
            .iter()
            .find(|(it, _)| it == &pc)
            .map(|(_, it)| it)
    }
    pub fn next_pc_of(&self, this_pc: ProgramCounter) -> Option<ProgramCounter> {
        let mut iter = self.instructions.iter().skip_while(|(pc, _)| *pc < this_pc);
        iter.next()
            .filter(|(pc, _)| *pc == this_pc)
            .and_then(|_| iter.next().map(|(it, _)| it).cloned())
    }
}

#[cfg(test)]
mod test {
    use crate::elements::instruction::Instruction;

    use super::MethodBody;
    use Instruction::*;

    impl Default for MethodBody {
        fn default() -> Self {
            Self {
                max_stack: Default::default(),
                max_locals: Default::default(),
                instructions: Default::default(),
                exception_table: Default::default(),
                line_number_table: Default::default(),
                local_variable_table: Default::default(),
                stack_map_table: Default::default(),
                runtime_visible_type_annotations: Default::default(),
                runtime_invisible_type_annotations: Default::default(),
            }
        }
    }

    #[test]
    fn instruction_at() {
        let body = MethodBody {
            instructions: vec![(0.into(), Nop), (1.into(), IConst0), (2.into(), IConst1)],
            ..Default::default()
        };
        assert_eq!(Some(&IConst0), body.instruction_at(1.into()));
    }

    #[test]
    fn next_insn() {
        let body = MethodBody {
            instructions: vec![
                (0.into(), Nop),
                (2.into(), IConst0),
                (3.into(), IConst1),
                (5.into(), Nop),
                (7.into(), IConst0),
                (9.into(), IConst1),
            ],
            ..Default::default()
        };
        assert_eq!(None, body.next_pc_of(1.into()));
        assert_eq!(None, body.next_pc_of(4.into()));
        assert_eq!(Some(2.into()), body.next_pc_of(0.into()));
        assert_eq!(Some(7.into()), body.next_pc_of(5.into()));
    }
}

#[derive(Debug)]
pub struct ExceptionTableEntry {
    pub start_pc: ProgramCounter,
    pub end_pc: ProgramCounter,
    pub handler_pc: ProgramCounter,
    pub catch_type: Option<ClassReference>,
}

impl ExceptionTableEntry {
    pub fn covers(&self, pc: ProgramCounter) -> bool {
        self.start_pc <= pc && pc <= self.end_pc
    }
}

#[derive(Debug)]
pub struct LineNumberTableEntry {
    pub start_pc: ProgramCounter,
    pub line_number: u16,
}

#[derive(Debug)]
pub struct LocalVariableTable {
    entries: HashMap<LocalVariableId, LocalVariableTableEntry>,
}

impl LocalVariableTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn merge_desc_attr(&mut self, attrs: Vec<LocalVariableDescAttr>) {
        for LocalVariableDescAttr { key, field_type } in attrs.into_iter() {
            let entry = self.entries.entry(key).or_default();
            entry.var_type = Some(field_type);
        }
    }
    pub(crate) fn merge_type_attr(&mut self, attrs: Vec<LocalVariableTypeAttr>) {
        for LocalVariableTypeAttr { key, signature } in attrs.into_iter() {
            let entry = self.entries.entry(key).or_default();
            entry.signature = Some(signature);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct LocalVariableId {
    pub start_pc: ProgramCounter,
    pub length: ProgramCounter,
    pub index: u16,
    pub name: String,
}

#[derive(Debug, Default)]
pub struct LocalVariableTableEntry {
    pub var_type: Option<FieldType>,
    pub signature: Option<String>,
}

#[derive(Debug)]
pub enum VerificationTypeInfo {
    TopVariable,
    IntegerVariable,
    FloatVariable,
    NullVariable,
    UninitializedThisVariable,
    ObjectVariable(ClassReference),
    UninitializedVariable { offset: u16 },
    LongVariable,
    DoubleVariable,
}

#[derive(Debug)]
pub enum StackMapFrame {
    SameFrame {
        offset_delta: u16,
    },
    SameLocals1StackItemFrame(VerificationTypeInfo),
    Semantics1StackItemFrameExtended(u16, VerificationTypeInfo),
    ChopFrame {
        chop_count: u8,
        offset_delta: u16,
    },
    SameFrameExtended {
        offset_delta: u16,
    },
    AppendFrame {
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
    },
    FullFrame {
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
        stack: Vec<VerificationTypeInfo>,
    },
}
