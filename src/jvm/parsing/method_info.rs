use std::str::FromStr;

use crate::{
    jvm::class::ClassFileParsingError,
    jvm::{
        class::{ClassReference, ClassVersion},
        code::{
            ExceptionTableEntry, Instruction, LineNumberTableEntry, LocalVariableTable, MethodBody,
            StackMapFrame,
        },
        method::{
            Method, MethodAccessFlags, MethodDescriptor, MethodParameter,
            MethodParameterAccessFlags, CLASS_INITIALIZER_NAME,
        },
        parsing::parsing_context::ParsingContext,
    },
    macros::extract_attributes,
};

use super::{
    attribute::Attribute,
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    reader_utils::{parse_multiple, read_bytes_vec, read_u16, read_u32, read_u8},
};

impl ExceptionTableEntry {
    fn parse<R>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let start_pc = read_u16(reader)?.into();
        let end_pc = read_u16(reader)?.into();
        let covered_pc = start_pc..=end_pc;
        let handler_pc = read_u16(reader)?.into();
        let catch_type_idx = read_u16(reader)?;
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

impl Attribute {
    pub(super) fn parse_line_no_table<R>(
        reader: &mut R,
        _ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
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
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let max_stack = read_u16(reader)?;
        let max_locals = read_u16(reader)?;
        let code_length = read_u32(reader)?;

        let code = read_bytes_vec(reader, code_length as usize)?;
        let instructions = Instruction::parse_code(code, ctx)?;

        // exception table
        let exception_table_len = read_u16(reader)?;
        let mut exception_table = Vec::with_capacity(exception_table_len as usize);
        for _ in 0..exception_table_len {
            let entry = ExceptionTableEntry::parse(reader, ctx)?;
            exception_table.push(entry);
        }

        let attributes = parse_multiple(reader, ctx, Attribute::parse)?;
        let mut local_variable_table = None;
        extract_attributes! {
            for attributes in "code" by {
                let line_number_table <= LineNumberTable,
                let stack_map_table <= StackMapTable,
                let runtime_visible_type_annotations <= RuntimeVisibleTypeAnnotations,
                let runtime_invisible_type_annotations <= RuntimeInvisibleTypeAnnotations,
                match Attribute::LocalVariableTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTable::new());
                    for LocalVariableDescAttr { id, name, field_type } in it {
                        table.merge_type(id, name, field_type)?;
                    }
                },
                match Attribute::LocalVariableTypeTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTable::new());
                    for LocalVariableTypeAttr { id, name, signature } in it {
                        table.merge_signature(id, name, signature)?;
                    }
                },
            }
        }

        Ok(Attribute::Code(MethodBody {
            max_stack,
            max_locals,
            exception_table,
            instructions,
            line_number_table,
            local_variable_table,
            stack_map_table,
            runtime_visible_type_annotations: runtime_visible_type_annotations.unwrap_or_default(),
            runtime_invisible_type_annotations: runtime_invisible_type_annotations
                .unwrap_or_default(),
        }))
    }
    pub(super) fn parse_local_variable_table<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let table_len = read_u16(reader)?;
        let mut local_variable_table = Vec::with_capacity(table_len as usize);
        for _ in 0..table_len {
            let entry = LocalVariableDescAttr::parse(reader, ctx)?;
            local_variable_table.push(entry);
        }
        Ok(Attribute::LocalVariableTable(local_variable_table))
    }

    pub(super) fn parse_local_variable_type_table<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let table_len = read_u16(reader)?;
        let mut local_variable_type_table = Vec::with_capacity(table_len as usize);
        for _ in 0..table_len {
            let entry = LocalVariableTypeAttr::parse(reader, ctx)?;
            local_variable_type_table.push(entry);
        }
        Ok(Attribute::LocalVariableTypeTable(local_variable_type_table))
    }

    pub(super) fn parse_stack_map_table<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let num_entries = read_u16(reader)?;
        let mut stack_map_table = Vec::with_capacity(num_entries as usize);
        for _ in 0..num_entries {
            let entry = StackMapFrame::parse(reader, ctx)?;
            stack_map_table.push(entry);
        }
        Ok(Self::StackMapTable(stack_map_table))
    }
    pub(super) fn parse_exceptions<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let number_of_exceptions = read_u16(reader)?;
        let mut exceptions = Vec::with_capacity(number_of_exceptions as usize);
        for _ in 0..number_of_exceptions {
            let exception_index = read_u16(reader)?;
            let exception = ctx.constant_pool.get_class_ref(exception_index)?;
            exceptions.push(exception);
        }
        Ok(Self::Exceptions(exceptions))
    }

    pub(super) fn parse_method_parameters<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let parameters_count = read_u8(reader)?;
        let mut parameters = Vec::with_capacity(parameters_count as usize);
        for _ in 0..parameters_count {
            let name_index = read_u16(reader)?;
            let name = ctx.constant_pool.get_str(name_index)?.to_owned();
            let access_flag_bits = read_u16(reader)?;
            let Some(access_flags) = MethodParameterAccessFlags::from_bits(access_flag_bits) else {
                return Err(ClassFileParsingError::UnknownFlags(
                    access_flag_bits,
                    "method_parameter",
                ));
            };
            parameters.push(MethodParameter { name, access_flags });
        }
        Ok(Self::MethodParameters(parameters))
    }
}

impl Method {
    pub(crate) fn parse<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let access = read_u16(reader)?;
        let Some(access_flags) = MethodAccessFlags::from_bits(access) else {
            return Err(ClassFileParsingError::UnknownFlags(access, "method"));
        };
        let name_index = read_u16(reader)?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let descriptor_index = read_u16(reader)?;
        let descriptor = ctx.constant_pool.get_str(descriptor_index)?;
        let descriptor = MethodDescriptor::from_str(descriptor)?;
        let owner = ClassReference {
            binary_name: ctx.current_class_binary_name.clone(),
        };

        let attributes = parse_multiple(reader, ctx, Attribute::parse)?;
        extract_attributes! {
            for attributes in "method_info" by {
                let body <= Code,
                let exceptions <= Exceptions,
                let rt_visible_anno <= RuntimeVisibleAnnotations,
                let rt_invisible_anno <= RuntimeInvisibleAnnotations,
                let rt_visible_type_anno <= RuntimeVisibleTypeAnnotations,
                let rt_invisible_type_anno <= RuntimeInvisibleTypeAnnotations,
                let rt_visible_param_anno <= RuntimeVisibleParameterAnnotations,
                let rt_invisible_param_anno <= RuntimeInvisibleParameterAnnotations,
                let annotation_default <= AnnotationDefault,
                let method_parameters <= MethodParameters,
                let signature <= Signature,
                if Synthetic => is_synthetic = true,
                if Deprecated => is_deprecated = true,
            }
        };

        // JVM specification 4.7.3
        // If the method is either `native` or `abstract`, and is not a class or interface initialization method
        if (access_flags.contains(MethodAccessFlags::NATIVE)
            || access_flags.contains(MethodAccessFlags::ABSTRACT))
            && name != CLASS_INITIALIZER_NAME
        {
            // then its method_info structure must not have a Code attribute in its attributes table
            if body.is_some() {
                Err(ClassFileParsingError::MalformedClassFile(
                    "Unexpected code attribute",
                ))?
            }
        } else {
            // Otherwise, its method_info structure must have exactly one Code attribute in its attributes table
            if body.is_none() {
                Err(ClassFileParsingError::MalformedClassFile(
                    "The method must have a body",
                ))?
            }
        }

        if ctx.class_version
            >= (ClassVersion {
                major: 51,
                minor: 0,
            })
            && name == CLASS_INITIALIZER_NAME
        {
            // In a class file whose version number is 51.0 or above, the method has its ACC_STATIC flag set and takes no arguments (§4.6).
            if !access_flags.contains(MethodAccessFlags::STATIC)
                || !descriptor.parameters_types.is_empty()
            {
                Err(ClassFileParsingError::MalformedClassFile("Class initializer in class version 51 or above must be static and takes no arguments"))?
            }
        }

        Ok(Method {
            access_flags,
            name,
            descriptor,
            owner,
            body,
            excaptions: exceptions.unwrap_or_default(),
            runtime_visible_annotations: rt_visible_anno.unwrap_or_default(),
            runtime_invisible_annotations: rt_invisible_anno.unwrap_or_default(),
            runtime_visible_type_annotations: rt_visible_type_anno.unwrap_or_default(),
            runtime_invisible_type_annotations: rt_invisible_type_anno.unwrap_or_default(),
            runtime_visible_parameter_annotations: rt_visible_param_anno.unwrap_or_default(),
            runtime_invisible_parameter_annotations: rt_invisible_param_anno.unwrap_or_default(),
            annotation_default,
            parameters: method_parameters.unwrap_or_default(),
            is_synthetic,
            is_deprecated,
            signature,
        })
    }
}
