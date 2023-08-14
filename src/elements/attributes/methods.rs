use crate::{
    elements::{
        class_file::{ClassFileParsingError, ClassReference},
        constant_pool::ConstantPool,
    },
    utils::{read_bytes_vec, read_u16, read_u32},
};

use super::{Attribute, AttributeList};

#[derive(Debug)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: ClassReference,
}

impl ExceptionTableEntry {
    fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<ExceptionTableEntry, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let end_pc = read_u16(reader)?;
        let handler_pc = read_u16(reader)?;
        let catch_type_idx = read_u16(reader)?;
        let catch_type = constant_pool.get_class_ref(catch_type_idx)?;
        Ok(ExceptionTableEntry {
            start_pc,
            end_pc,
            handler_pc,
            catch_type,
        })
    }
}

#[derive(Debug)]
pub struct LineNumberTableEntry {
    pub start_pc: u16,
    pub line_number: u16,
}
impl LineNumberTableEntry {
    fn parse<R>(reader: &mut R) -> Result<LineNumberTableEntry, ClassFileParsingError>
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
    fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<LocalVariableTableEntry, ClassFileParsingError>
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
    fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<LocalVariableTypeTableEntry, ClassFileParsingError>
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
pub struct StackMapTableEntry {}

#[derive(Debug)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub line_number_table: Option<Vec<LineNumberTableEntry>>,
    pub local_variable_table: Option<Vec<LocalVariableTableEntry>>,
    pub local_variable_type_table: Option<Vec<LocalVariableTypeTableEntry>>,
    pub stack_map_table: Option<Vec<StackMapTableEntry>>,
}

impl Attribute {
    pub(super) fn parse_line_no_table<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let line_number_table_len = read_u16(reader)?;
        let mut line_number_table = Vec::with_capacity(line_number_table_len as usize);
        for _ in 0..line_number_table_len {
            let entry = LineNumberTableEntry::parse(reader)?;
            line_number_table.push(entry);
        }
        Ok(Attribute::LineNumberTable(line_number_table))
    }

    pub(super) fn parse_code<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let max_stack = read_u16(reader)?;
        let max_locals = read_u16(reader)?;
        let code_length = read_u32(reader)?;

        let code = read_bytes_vec(reader, code_length as usize)?;

        // exception table
        let exception_table_len = read_u16(reader)?;
        let mut exception_table = Vec::with_capacity(exception_table_len as usize);
        for _ in 0..exception_table_len {
            let entry = ExceptionTableEntry::parse(reader, constant_pool)?;
            exception_table.push(entry);
        }

        let attributes = AttributeList::parse(reader, constant_pool)?;
        let mut line_number_table = None;
        let mut local_variable_table = None;
        let mut local_variable_type_table = None;
        let mut stack_map_table = None;

        for attr in attributes.into_iter() {
            match attr {
                Attribute::LineNumberTable(it) => line_number_table = Some(it),
                Attribute::LocalVariableTable(it) => local_variable_table = Some(it),
                Attribute::LocalVariableTypeTable(it) => local_variable_type_table = Some(it),
                Attribute::StackMapTable(it) => stack_map_table = Some(it),
                _ => return Err(ClassFileParsingError::UnexpectedAttribute),
            }
        }

        Ok(Attribute::Code(Code {
            max_stack,
            max_locals,
            exception_table,
            code,
            line_number_table,
            local_variable_table,
            local_variable_type_table,
            stack_map_table,
        }))
    }
    pub(super) fn parse_local_variable_table<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let table_len = read_u16(reader)?;
        let mut local_variable_table = Vec::with_capacity(table_len as usize);
        for _ in 0..table_len {
            let entry = LocalVariableTableEntry::parse(reader, constant_pool)?;
            local_variable_table.push(entry);
        }
        Ok(Attribute::LocalVariableTable(local_variable_table))
    }

    pub(super) fn parse_local_variable_type_table<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let table_len = read_u16(reader)?;
        let mut local_variable_type_table = Vec::with_capacity(table_len as usize);
        for _ in 0..table_len {
            let entry = LocalVariableTypeTableEntry::parse(reader, constant_pool)?;
            local_variable_type_table.push(entry);
        }
        Ok(Attribute::LocalVariableTypeTable(local_variable_type_table))
    }
}
