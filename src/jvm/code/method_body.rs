use std::{
    collections::{BTreeMap, HashMap},
    ops::RangeInclusive,
};

use crate::{
    jvm::{annotation::TypeAnnotation, class::ClassReference},
    types::FieldType,
};

use super::{Instruction, ProgramCounter};

#[derive(Debug)]
pub struct MethodBody {
    pub max_stack: u16,
    pub max_locals: u16,
    pub instructions: InstructionList,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub line_number_table: Option<Vec<LineNumberTableEntry>>,
    pub local_variable_table: Option<LocalVariableTable>,
    pub stack_map_table: Option<Vec<StackMapFrame>>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
}

impl MethodBody {
    pub fn instruction_at(&self, pc: ProgramCounter) -> Option<&Instruction> {
        self.instructions.get(&pc)
    }
}

pub type InstructionList = BTreeMap<ProgramCounter, Instruction>;

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use crate::jvm::code::Instruction;

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
            instructions: BTreeMap::from([
                (0.into(), Nop),
                (1.into(), IConst0),
                (2.into(), IConst1),
            ]),
            ..Default::default()
        };
        assert_eq!(Some(&IConst0), body.instruction_at(1.into()));
    }
}

#[derive(Debug, Clone)]
pub struct ExceptionTableEntry {
    pub covered_pc: RangeInclusive<ProgramCounter>,
    pub handler_pc: ProgramCounter,
    pub catch_type: Option<ClassReference>,
}

impl ExceptionTableEntry {
    pub fn covers(&self, pc: ProgramCounter) -> bool {
        self.covered_pc.contains(&pc)
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

impl Default for LocalVariableTable {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalVariableTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn merge_type(&mut self, key: LocalVariableId, field_type: FieldType) {
        let entry = self.entries.entry(key).or_default();
        entry.var_type = Some(field_type);
    }

    pub(crate) fn merge_signature(&mut self, key: LocalVariableId, signature: String) {
        let entry = self.entries.entry(key).or_default();
        entry.signature = Some(signature);
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
