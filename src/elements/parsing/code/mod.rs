pub(super) mod instruction_impl;
pub(super) mod stack_map;

use crate::{
    elements::instruction::{LineNumberTableEntry, LocalVariableKey, VerificationTypeInfo},
    errors::ClassFileParsingError,
    reader_utils::{read_u16, read_u8},
    types::FieldType,
};

use super::constant_pool::ParsingContext;

#[derive(Debug)]
pub(crate) struct LocalVariableDescAttr {
    pub key: LocalVariableKey,
    pub name: String,
    pub field_type: FieldType,
}

#[derive(Debug)]
pub(crate) struct LocalVariableTypeAttr {
    pub key: LocalVariableKey,
    pub name: String,
    pub signature: String,
}

impl LineNumberTableEntry {
    pub(super) fn parse<R>(reader: &mut R) -> Result<LineNumberTableEntry, ClassFileParsingError>
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
        ctx: &ParsingContext,
    ) -> Result<LocalVariableDescAttr, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let length = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let name = ctx.get_str(&name_index)?.to_owned();
        let descriptor_index = read_u16(reader)?;
        let descriptor = ctx.get_str(&descriptor_index)?;
        let field_type = FieldType::try_from(descriptor)?;
        let index = read_u16(reader)?;
        let key = LocalVariableKey {
            start_pc,
            length,
            index,
        };
        Ok(LocalVariableDescAttr {
            key,
            name,
            field_type,
        })
    }
}

impl LocalVariableTypeAttr {
    pub(super) fn parse<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<LocalVariableTypeAttr, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let length = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let name = ctx.get_str(&name_index)?.to_owned();
        let signature_index = read_u16(reader)?;
        let signature = ctx.get_str(&signature_index)?.to_owned();
        let index = read_u16(reader)?;
        let key = LocalVariableKey {
            start_pc,
            length,
            index,
        };
        Ok(LocalVariableTypeAttr {
            key,
            name,
            signature,
        })
    }
}

impl VerificationTypeInfo {
    pub(super) fn parse<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<VerificationTypeInfo, ClassFileParsingError>
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
                let class_ref = ctx.get_class_ref(&cpool_index)?;
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
