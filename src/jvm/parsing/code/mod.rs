pub(super) mod instruction_impl;
pub(super) mod raw_instruction;
pub(super) mod stack_map;

use std::str::FromStr;

use crate::{
    jvm::{
        code::{
            ExceptionTableEntry, Instruction, LineNumberTableEntry, LocalVariableId,
            LocalVariableTable, MethodBody,
        },
        method::ParameterInfo,
    },
    macros::extract_attributes,
    types::field_type::FieldType,
};

use super::{
    jvm_element_parser::{parse_flags, parse_jvm, ParseJvmElement},
    parsing_context::ParsingContext,
    reader_utils::{read_byte_chunk, ClassReader},
    Error,
};

#[derive(Debug)]
pub(crate) struct LocalVariableDescAttr {
    pub id: LocalVariableId,
    pub name: String,
    pub field_type: FieldType,
}

#[derive(Debug)]
pub(crate) struct LocalVariableTypeAttr {
    pub id: LocalVariableId,
    pub name: String,
    pub signature: String,
}

impl<R: std::io::Read> ParseJvmElement<R> for LineNumberTableEntry {
    fn parse(reader: &mut R, _ctx: &ParsingContext) -> Result<Self, Error> {
        let start_pc = reader.read_value::<u16>()?.into();
        let line_number = reader.read_value()?;
        Ok(Self {
            start_pc,
            line_number,
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ExceptionTableEntry {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let start_pc = reader.read_value::<u16>()?.into();
        let end_pc = reader.read_value::<u16>()?.into();
        let covered_pc = start_pc..=end_pc;
        let handler_pc = reader.read_value::<u16>()?.into();
        let catch_type_idx = reader.read_value()?;
        let catch_type = if catch_type_idx == 0 {
            None
        } else {
            Some(ctx.constant_pool.get_class_ref(catch_type_idx)?)
        };
        Ok(ExceptionTableEntry {
            covered_pc,
            handler_pc,
            catch_type,
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for LocalVariableDescAttr {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let start_pc = reader.read_value::<u16>()?;
        let length = reader.read_value::<u16>()?;
        let effective_range = start_pc.into()..(start_pc + length).into();
        let name_index = reader.read_value()?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let descriptor_index = reader.read_value()?;
        let descriptor = ctx.constant_pool.get_str(descriptor_index)?;
        let field_type = FieldType::from_str(descriptor)?;
        let index = reader.read_value()?;
        let id = LocalVariableId {
            effective_range,
            index,
        };
        Ok(LocalVariableDescAttr {
            id,
            name,
            field_type,
        })
    }
}
impl<R: std::io::Read> ParseJvmElement<R> for LocalVariableTypeAttr {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let start_pc = reader.read_value::<u16>()?;
        let length = reader.read_value::<u16>()?;
        let effective_range = start_pc.into()..(start_pc + length).into();
        let name_index = reader.read_value()?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let signature_index = reader.read_value()?;
        let signature = ctx.constant_pool.get_str(signature_index)?.to_owned();
        let index = reader.read_value()?;
        let id = LocalVariableId {
            effective_range,
            index,
        };
        Ok(LocalVariableTypeAttr {
            id,
            name,
            signature,
        })
    }
}
impl<R: std::io::Read> ParseJvmElement<R> for ParameterInfo {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let name_index = reader.read_value()?;
        let name = if name_index == 0 {
            None
        } else {
            Some(ctx.constant_pool.get_str(name_index)?.to_owned())
        };
        let access_flags = parse_flags(reader)?;
        Ok(ParameterInfo { name, access_flags })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for MethodBody {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let max_stack = reader.read_value()?;
        let max_locals = reader.read_value()?;
        let code_length: u32 = reader.read_value()?;

        let code = read_byte_chunk(reader, code_length as usize)?;
        let instructions = Instruction::parse_code(code, ctx)?;

        let exception_table = parse_jvm!(u16, reader, ctx)?;
        let attributes: Vec<Attribute> = parse_jvm!(u16, reader, ctx)?;
        let mut local_variable_table = None;
        extract_attributes! {
            for attributes in "code" by {
                let line_number_table: LineNumberTable,
                let stack_map_table: StackMapTable,
                let runtime_visible_type_annotations: RuntimeVisibleTypeAnnotations unwrap_or_default,
                let runtime_invisible_type_annotations: RuntimeInvisibleTypeAnnotations unwrap_or_default,
                match Attribute::LocalVariableTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTable::default());
                    for LocalVariableDescAttr { id, name, field_type } in it {
                        table.merge_type(id, name, field_type)?;
                    }
                },
                match Attribute::LocalVariableTypeTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTable::default());
                    for LocalVariableTypeAttr { id, name, signature } in it {
                        table.merge_signature(id, name, signature)?;
                    }
                },
            }
        }

        Ok(Self {
            max_stack,
            max_locals,
            instructions,
            exception_table,
            line_number_table,
            local_variable_table,
            stack_map_table,
            runtime_visible_type_annotations,
            runtime_invisible_type_annotations,
        })
    }
}
