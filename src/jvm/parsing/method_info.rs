use std::str::FromStr;

use crate::{
    jvm::ClassFileParsingError,
    jvm::{
        class::{ClassReference, ClassVersion},
        instruction::{
            ExceptionTableEntry, Instruction, LineNumberTableEntry, LocalVariableTable, MethodBody,
            StackMapFrame,
        },
        method::{
            Method, MethodAccessFlags, MethodDescriptor, MethodParameter,
            MethodParameterAccessFlags, CLASS_INITIALIZER_NAME,
        },
        parsing::parsing_context::ParsingContext,
    },
    macros::fill_once,
};

use super::{
    attribute::Attribute,
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    reader_utils::{parse_multiple, read_bytes_vec, read_u16, read_u32, read_u8},
};

impl ExceptionTableEntry {
    fn parse<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<ExceptionTableEntry, ClassFileParsingError>
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
    ) -> Result<Attribute, ClassFileParsingError>
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
    ) -> Result<Attribute, ClassFileParsingError>
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

        let attributes = parse_multiple(reader, &ctx, Attribute::parse)?;
        let mut line_number_table = None;
        let mut local_variable_table = None;
        let mut stack_map_table = None;
        let mut runtime_visible_type_annotations = None;
        let mut runtime_invisible_type_annotations = None;

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
                Attribute::RuntimeVisibleTypeAnnotations(it) => {
                    runtime_visible_type_annotations = Some(it)
                }
                Attribute::RuntimeInvisibleTypeAnnotations(it) => {
                    runtime_invisible_type_annotations = Some(it)
                }
                it => Err(ClassFileParsingError::UnexpectedAttribute(
                    it.name(),
                    "code",
                ))?,
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
            runtime_visible_type_annotations: runtime_visible_type_annotations.unwrap_or_default(),
            runtime_invisible_type_annotations: runtime_invisible_type_annotations
                .unwrap_or_default(),
        }))
    }
    pub(super) fn parse_local_variable_table<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Attribute, ClassFileParsingError>
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
    ) -> Result<Attribute, ClassFileParsingError>
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
    ) -> Result<Attribute, ClassFileParsingError>
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
    ) -> Result<Attribute, ClassFileParsingError>
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
        let mut body = None;
        let mut exceptions = None;
        let mut rt_visible_anno = None;
        let mut rt_invisible_anno = None;
        let mut rt_visible_type_anno = None;
        let mut rt_invisible_type_anno = None;
        let mut rt_visible_param_anno = None;
        let mut rt_invisible_param_anno = None;
        let mut annotation_default = None;
        let mut method_parameters = None;
        let mut is_synthetic = false;
        let mut is_deprecated = false;
        let mut signature = None;
        for attr in attributes.into_iter() {
            use Attribute::*;
            match attr {
                Code(b) => fill_once!(body, b, "code"),
                Exceptions(ex) => fill_once!(exceptions, ex, "exception table"),
                RuntimeVisibleAnnotations(it) => {
                    fill_once!(rt_visible_anno, it, "RuntimeVisibleAnnotations")
                }
                RuntimeInvisibleAnnotations(it) => {
                    fill_once!(rt_invisible_anno, it, "RuntimeInvisibleAnnotations")
                }
                RuntimeVisibleTypeAnnotations(it) => {
                    fill_once!(rt_visible_type_anno, it, "RuntimeVisibleTypeAnnotations")
                }
                RuntimeInvisibleTypeAnnotations(it) => fill_once!(
                    rt_invisible_type_anno,
                    it,
                    "RuntimeInvisibleTypeAnnotations"
                ),
                RuntimeVisibleParameterAnnotations(it) => fill_once!(
                    rt_visible_param_anno,
                    it,
                    "RuntimeVisibleParameterAnnotations"
                ),
                RuntimeInvisibleParameterAnnotations(it) => fill_once!(
                    rt_invisible_param_anno,
                    it,
                    "RuntimeInvisibleParameterAnnotations"
                ),
                AnnotationDefault(it) => fill_once!(annotation_default, it, "AnnotationDefault"),
                MethodParameters(mp) => fill_once!(method_parameters, mp, "method parameter table"),
                Synthetic => is_synthetic = true,
                Deprecated => is_deprecated = true,
                Signature(sig) => fill_once!(signature, sig, "signagure"),
                it => Err(ClassFileParsingError::UnexpectedAttribute(
                    it.name(),
                    "method_info",
                ))?,
            }
        }

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
