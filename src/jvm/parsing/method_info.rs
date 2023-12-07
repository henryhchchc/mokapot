use std::str::FromStr;

use crate::{
    jvm::{
        class::{ClassReference, ClassVersion},
        method::{Method, MethodAccessFlags, MethodDescriptor},
        parsing::{
            jvm_element_parser::{parse_flags, parse_jvm_element},
            parsing_context::ParsingContext,
        },
        ClassFileParsingError, ClassFileParsingResult,
    },
    macros::extract_attributes,
};

use super::{jvm_element_parser::ParseJvmElement, reader_utils::read_u16};

impl<R: std::io::Read> ParseJvmElement<R> for Method {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let access_flags: MethodAccessFlags = parse_flags(reader)?;
        let name_index = read_u16(reader)?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let descriptor_index = read_u16(reader)?;
        let descriptor = ctx.constant_pool.get_str(descriptor_index)?;
        let descriptor = MethodDescriptor::from_str(descriptor)?;
        let owner = ClassReference {
            binary_name: ctx.current_class_binary_name.clone(),
        };

        let attributes: Vec<Attribute> = parse_jvm_element(reader, ctx)?;
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
            && name != Method::CLASS_INITIALIZER_NAME
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
            && name == Method::CLASS_INITIALIZER_NAME
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
