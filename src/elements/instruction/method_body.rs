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
    pub instructions: HashMap<ProgramCounter, Instruction>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub line_number_table: Option<Vec<LineNumberTableEntry>>,
    pub local_variable_table: Option<LocalVariableTable>,
    pub stack_map_table: Option<Vec<StackMapFrame>>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
}

#[derive(Debug)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: Option<ClassReference>,
}

#[derive(Debug)]
pub struct LineNumberTableEntry {
    pub start_pc: u16,
    pub line_number: u16,
}

#[derive(Debug)]
pub struct LocalVariableTable {
    entries: HashMap<LocalVariableKey, LocalVariableTableEntry>,
}

impl LocalVariableTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn merge_desc_attr(&mut self, attrs: Vec<LocalVariableDescAttr>) {
        for LocalVariableDescAttr {
            key,
            name,
            field_type,
        } in attrs.into_iter()
        {
            let entry = self.entries.entry(key).or_default();
            entry.name = name;
            entry.var_type = Some(field_type);
        }
    }
    pub(crate) fn merge_type_attr(&mut self, attrs: Vec<LocalVariableTypeAttr>) {
        for LocalVariableTypeAttr {
            key,
            name,
            signature,
        } in attrs.into_iter()
        {
            let entry = self.entries.entry(key).or_default();
            entry.name = name;
            entry.signature = Some(signature);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
pub struct LocalVariableKey {
    pub start_pc: u16,
    pub length: u16,
    pub index: u16,
}

#[derive(Debug, Default)]
pub struct LocalVariableTableEntry {
    pub key: LocalVariableKey,
    pub name: String,
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
