use crate::{
    elements::{
        class_parser::{ClassFileParsingError, ClassFileParsingResult},
        instruction::Instruction,
        method::{
            ExceptionTableEntry, LineNumberTableEntry, LocalVariableDescAttr, LocalVariableTable,
            LocalVariableTypeAttr, MethodBody, MethodDescriptor, MethodParameter,
            MethodParameterAccessFlags, StackMapFrame, Method, MethodAccessFlags,
        },
        parsing::constant_pool::ConstantPool,
    },
    utils::{read_bytes_vec, read_u16, read_u32, read_u8},
};

use super::attribute::{Attribute, AttributeList};

impl ExceptionTableEntry {
    fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<ExceptionTableEntry>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?;
        let end_pc = read_u16(reader)?;
        let handler_pc = read_u16(reader)?;
        let catch_type_idx = read_u16(reader)?;
        let catch_type = constant_pool.get_class_ref(&catch_type_idx)?;
        Ok(ExceptionTableEntry {
            start_pc,
            end_pc,
            handler_pc,
            catch_type,
        })
    }
}

impl Attribute {
    pub(super) fn parse_line_no_table<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Attribute>
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
    ) -> ClassFileParsingResult<Attribute>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let max_stack = read_u16(reader)?;
        let max_locals = read_u16(reader)?;
        let code_length = read_u32(reader)?;

        let code = read_bytes_vec(reader, code_length as usize)?;
        let instructions = Instruction::parse_code(code, constant_pool)?;

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
        let mut stack_map_table = None;

        for attr in attributes.into_iter() {
            match attr {
                Attribute::LineNumberTable(it) => line_number_table = Some(it),
                Attribute::LocalVariableTable(it) => local_variable_table
                    .get_or_insert(LocalVariableTable::new())
                    .merge_desc_attr(it),
                Attribute::LocalVariableTypeTable(it) => local_variable_table
                    .get_or_insert(LocalVariableTable::new())
                    .merge_type_attr(it),
                Attribute::StackMapTable(it) => stack_map_table = Some(it),
                _ => return Err(ClassFileParsingError::UnexpectedAttribute),
            };
        }

        Ok(Attribute::Code(MethodBody {
            max_stack,
            max_locals,
            exception_table,
            instructions,
            line_number_table,
            local_variable_table,
            stack_map_table,
        }))
    }
    pub(super) fn parse_local_variable_table<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Attribute>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let table_len = read_u16(reader)?;
        let mut local_variable_table = Vec::with_capacity(table_len as usize);
        for _ in 0..table_len {
            let entry = LocalVariableDescAttr::parse(reader, constant_pool)?;
            local_variable_table.push(entry);
        }
        Ok(Attribute::LocalVariableTable(local_variable_table))
    }

    pub(super) fn parse_local_variable_type_table<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Attribute>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let table_len = read_u16(reader)?;
        let mut local_variable_type_table = Vec::with_capacity(table_len as usize);
        for _ in 0..table_len {
            let entry = LocalVariableTypeAttr::parse(reader, constant_pool)?;
            local_variable_type_table.push(entry);
        }
        Ok(Attribute::LocalVariableTypeTable(local_variable_type_table))
    }

    pub(super) fn parse_stack_map_table<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Attribute>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let num_entries = read_u16(reader)?;
        let mut stack_map_table = Vec::with_capacity(num_entries as usize);
        for _ in 0..num_entries {
            let entry = StackMapFrame::parse(reader, constant_pool)?;
            stack_map_table.push(entry);
        }
        Ok(Self::StackMapTable(stack_map_table))
    }
    pub(super) fn parse_exceptions<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Attribute>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let number_of_exceptions = read_u16(reader)?;
        let mut exceptions = Vec::with_capacity(number_of_exceptions as usize);
        for _ in 0..number_of_exceptions {
            let exception_index = read_u16(reader)?;
            let exception = constant_pool.get_class_ref(&exception_index)?;
            exceptions.push(exception);
        }
        Ok(Self::Exceptions(exceptions))
    }

    pub(super) fn parse_method_parameters<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let parameters_count = read_u8(reader)?;
        let mut parameters = Vec::with_capacity(parameters_count as usize);
        for _ in 0..parameters_count {
            let name_index = read_u16(reader)?;
            let name = constant_pool.get_string(&name_index)?;
            let access_flag_bits = read_u16(reader)?;
            let Some(access_flags) = MethodParameterAccessFlags::from_bits(access_flag_bits) else {
                return Err(ClassFileParsingError::UnknownFlags(access_flag_bits));
            };
            parameters.push(MethodParameter { name, access_flags });
        }
        Ok(Self::MethodParameters(parameters))
    }
}

impl Method {
    pub(crate) fn parse_multiple<R>(
        reader: &mut R,
        methods_count: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<Self>>
    where
        R: std::io::Read,
    {
        let mut methods = Vec::with_capacity(methods_count as usize);
        for _ in 0..methods_count {
            let method = Self::parse(reader, constant_pool)?;
            methods.push(method);
        }
        Ok(methods)
    }
    pub(crate) fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let access = read_u16(reader)?;
        let Some(access_flags) = MethodAccessFlags::from_bits(access) else {
            return Err(ClassFileParsingError::UnknownFlags(access));
        };
        let name_index = read_u16(reader)?;
        let name = constant_pool.get_string(&name_index)?;
        let descriptor_index = read_u16(reader)?;
        let descriptor = constant_pool.get_str(&descriptor_index)?;
        let descriptor = MethodDescriptor::from_descriptor(descriptor)?;

        let attributes = AttributeList::parse(reader, constant_pool)?;
        let mut body = None;
        let mut exceptions = None;
        let mut rt_visible_anno = None;
        let mut rt_invisible_anno = None;
        let mut rt_visible_type_anno = None;
        let mut rt_invisible_type_anno = None;
        let mut annotation_default = None;
        let mut method_parameters = None;
        let mut is_synthetic = false;
        let mut is_deprecated = false;
        let mut signature = None;
        for attr in attributes.into_iter() {
            match attr {
                Attribute::Code(b) => body = Some(b),
                Attribute::Exceptions(ex) => exceptions = Some(ex),
                Attribute::RuntimeVisibleAnnotations(rv) => rt_visible_anno = Some(rv),
                Attribute::RuntimeInvisibleAnnotations(ri) => rt_invisible_anno = Some(ri),
                Attribute::RuntimeVisibleTypeAnnotations(rt) => rt_visible_type_anno = Some(rt),
                Attribute::RuntimeInvisibleTypeAnnotations(rt) => rt_invisible_type_anno = Some(rt),
                Attribute::AnnotationDefault(ad) => annotation_default = Some(ad),
                Attribute::MethodParameters(mp) => method_parameters = Some(mp),
                Attribute::Synthetic => is_synthetic = true,
                Attribute::Deprecated => is_deprecated = true,
                Attribute::Signature(sig) => signature = Some(sig),
                _ => Err(ClassFileParsingError::UnexpectedAttribute)?,
            }
        }

        Ok(Method {
            access_flags,
            name,
            descriptor,
            body,
            excaptions: exceptions.unwrap_or_default(),
            runtime_visible_annotations: rt_visible_anno.unwrap_or_default(),
            runtime_invisible_annotations: rt_invisible_anno.unwrap_or_default(),
            runtime_visible_type_annotations: rt_visible_type_anno.unwrap_or_default(),
            runtime_invisible_type_annotations: rt_invisible_type_anno.unwrap_or_default(),
            annotation_default,
            parameters: method_parameters.unwrap_or_default(),
            is_synthetic,
            is_deprecated,
            signature,
        })
    }
}
