pub(super) mod instruction_impl;
pub(super) mod raw_instruction;
pub(super) mod stack_map;

use std::{io::Read, str::FromStr};

use crate::{
    jvm::{
        code::{
            ExceptionTableEntry, Instruction, LineNumberTableEntry, LocalVariableId,
            LocalVariableTable, MethodBody, ProgramCounter,
        },
        method::ParameterInfo,
    },
    macros::extract_attributes,
    types::field_type::FieldType,
};

use super::{
    jvm_element_parser::{parse_flags, FromRaw, JvmElement},
    raw_attributes::{self, Code},
    reader_utils::ValueReaderExt,
    Context, Error,
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

impl JvmElement for LineNumberTableEntry {
    fn parse<R: Read + ?Sized>(reader: &mut R, _ctx: &Context) -> Result<Self, Error> {
        let start_pc = reader.read_value()?;
        let line_number = reader.read_value()?;
        Ok(Self {
            start_pc,
            line_number,
        })
    }
}

impl FromRaw for ExceptionTableEntry {
    type Raw = raw_attributes::ExceptionTableEntry;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let raw_attributes::ExceptionTableEntry {
            start_pc,
            end_pc,
            handler_pc,
            catch_type_idx,
        } = raw;
        let start_pc = ProgramCounter::from(start_pc);
        let end_pc = ProgramCounter::from(end_pc);
        let covered_pc = start_pc..=end_pc;
        let handler_pc = ProgramCounter::from(handler_pc);
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

impl JvmElement for LocalVariableDescAttr {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let effective_range = {
            let start_pc: ProgramCounter = reader.read_value()?;
            let length = reader.read_value::<u16>()?;
            let end_pc = start_pc.offset(i32::from(length))?;
            start_pc..end_pc
        };
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
impl JvmElement for LocalVariableTypeAttr {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let effective_range = {
            let start_pc: ProgramCounter = reader.read_value()?;
            let length = reader.read_value::<u16>()?;
            let end_pc = start_pc.offset(i32::from(length))?;
            start_pc..end_pc
        };
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
impl JvmElement for ParameterInfo {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
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

impl FromRaw for MethodBody {
    type Raw = Code;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Code {
            max_stack,
            max_locals,
            instruction_bytes: code,
            exception_table,
            attributes,
        } = raw;
        let instructions = Instruction::parse_code(code, ctx)?;

        let exception_table = exception_table
            .into_iter()
            .map(|it| FromRaw::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;
        let attributes: Vec<Attribute> = attributes
            .into_iter()
            .map(|it| FromRaw::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;
        let mut local_variable_table = None;
        extract_attributes! {
            for attributes in "code" {
                let line_number_table: LineNumberTable,
                let stack_map_table: StackMapTable,
                let runtime_visible_type_annotations:
                    RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations:
                    RuntimeInvisibleTypeAnnotations as unwrap_or_default,
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
                else let free_attributes
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
            free_attributes,
        })
    }
}
