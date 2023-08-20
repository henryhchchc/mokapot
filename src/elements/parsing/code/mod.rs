pub(super) mod instruction_impl;
pub(super) mod stack_map;

use crate::{
    elements::{
        class_parser::{ClassFileParsingError, ClassFileParsingResult},
        field::FieldType,
        method::{
            LineNumberTableEntry, LocalVariableDescAttr, LocalVariableTypeAttr,
            VerificationTypeInfo,
        },
    },
    utils::{read_u16, read_u8},
};

use super::constant_pool::ConstantPool;

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

impl LocalVariableDescAttr {
    pub(super) fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<LocalVariableDescAttr>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let length = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let name = constant_pool.get_string(&name_index)?;
        let descriptor_index = read_u16(reader)?;
        let descriptor = constant_pool.get_str(&descriptor_index)?;
        let field_type = FieldType::new(descriptor)?;
        let index = read_u16(reader)?;
        Ok(LocalVariableDescAttr {
            start_pc,
            length,
            name,
            field_type,
            index,
        })
    }
}

impl LocalVariableTypeAttr {
    pub(super) fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<LocalVariableTypeAttr>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let length = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let name = constant_pool.get_string(&name_index)?;
        let signature_index = read_u16(reader)?;
        let signature = constant_pool.get_string(&signature_index)?;
        let index = read_u16(reader)?;
        Ok(LocalVariableTypeAttr {
            start_pc,
            length,
            name,
            signature,
            index,
        })
    }
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
                let class_ref = constant_pool.get_class_ref(&cpool_index)?;
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
