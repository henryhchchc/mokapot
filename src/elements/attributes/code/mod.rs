pub(crate) mod instruction_impl;
pub(crate) mod instructions;

use crate::{
    elements::{
        class_file::{ClassFileParsingError, ClassFileParsingResult, ClassReference},
        constant_pool::ConstantPool,
    },
    utils::{read_u16, read_u8},
};

use super::Attribute;

#[derive(Debug)]
pub struct LineNumberTableEntry {
    pub start_pc: u16,
    pub line_number: u16,
}
impl LineNumberTableEntry {
    pub(super) fn parse<R>(reader: &mut R) -> ClassFileParsingResult<LineNumberTableEntry>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let line_number = read_u16(reader)?;
        Ok(LineNumberTableEntry {
            start_pc,
            line_number,
        })
    }
}

#[derive(Debug)]
pub struct LocalVariableTableEntry {
    pub start_pc: u16,
    pub length: u16,
    pub name: String,
    pub descriptor: String,
    pub index: u16,
}
impl LocalVariableTableEntry {
    pub(super) fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<LocalVariableTableEntry>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let length = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let name = constant_pool.get_string(name_index)?;
        let descriptor_index = read_u16(reader)?;
        let descriptor = constant_pool.get_string(descriptor_index)?;
        let index = read_u16(reader)?;
        Ok(LocalVariableTableEntry {
            start_pc,
            length,
            name,
            descriptor,
            index,
        })
    }
}

#[derive(Debug)]
pub struct LocalVariableTypeTableEntry {
    pub start_pc: u16,
    pub length: u16,
    pub name: String,
    pub signature: String,
    pub index: u16,
}
impl LocalVariableTypeTableEntry {
    pub(super) fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<LocalVariableTypeTableEntry>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let length = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let name = constant_pool.get_string(name_index)?;
        let signature_index = read_u16(reader)?;
        let signature = constant_pool.get_string(signature_index)?;
        let index = read_u16(reader)?;
        Ok(LocalVariableTypeTableEntry {
            start_pc,
            length,
            name,
            signature,
            index,
        })
    }
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
impl VerificationTypeInfo {
    pub(super) fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<VerificationTypeInfo>
    where
        R: std::io::Read,
    {
        let tag = read_u8(reader)?;
        let result = match tag {
            0 => Self::TopVariable,
            1 => Self::IntegerVariable,
            2 => Self::FloatVariable,
            3 => Self::DoubleVariable,
            4 => Self::LongVariable,
            5 => Self::NullVariable,
            6 => Self::UninitializedThisVariable,
            7 => {
                let cpool_index = read_u16(reader)?;
                let class_ref = constant_pool.get_class_ref(cpool_index)?;
                Self::ObjectVariable(class_ref)
            }
            8 => {
                let offset = read_u16(reader)?;
                Self::UninitializedVariable { offset }
            }
            _ => Err(ClassFileParsingError::InvalidVerificationTypeInfoTag(tag))?,
        };
        Ok(result)
    }
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
impl StackMapFrame {
    pub(super) fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<StackMapFrame>
    where
        R: std::io::Read,
    {
        let frame_type = read_u8(reader)?;
        let result = match frame_type {
            0..=63 => Self::SameFrame {
                offset_delta: frame_type as u16,
            },
            64..=127 => {
                Self::SameLocals1StackItemFrame(VerificationTypeInfo::parse(reader, constant_pool)?)
            }
            247 => {
                let offset_delta = read_u16(reader)?;
                let stack = VerificationTypeInfo::parse(reader, constant_pool)?;
                Self::Semantics1StackItemFrameExtended(offset_delta, stack)
            }
            248..=250 => {
                let chop_count = 251 - frame_type;
                let offset_delta = read_u16(reader)?;
                Self::ChopFrame {
                    chop_count,
                    offset_delta,
                }
            }
            251 => {
                let offset_delta = read_u16(reader)?;
                Self::SameFrameExtended { offset_delta }
            }
            252..=254 => {
                let offset_delta = read_u16(reader)?;
                let locals_count = frame_type - 251;
                let mut locals = Vec::with_capacity(locals_count as usize);
                for _ in 0..locals_count {
                    let local = VerificationTypeInfo::parse(reader, constant_pool)?;
                    locals.push(local);
                }
                Self::AppendFrame {
                    offset_delta,
                    locals,
                }
            }
            255 => {
                let offset_delta = read_u16(reader)?;
                let locals_count = read_u16(reader)?;
                let mut locals = Vec::with_capacity(locals_count as usize);
                for _ in 0..locals_count {
                    let local = VerificationTypeInfo::parse(reader, constant_pool)?;
                    locals.push(local);
                }
                let stacks_count = read_u16(reader)?;
                let mut stack = Vec::with_capacity(stacks_count as usize);
                for _ in 0..stacks_count {
                    let stack_element = VerificationTypeInfo::parse(reader, constant_pool)?;
                    stack.push(stack_element)
                }
                Self::FullFrame {
                    offset_delta,
                    locals,
                    stack,
                }
            }
            _ => Err(ClassFileParsingError::UnknownStackMapFrameType(frame_type))?,
        };
        Ok(result)
    }
}
impl Attribute {}
