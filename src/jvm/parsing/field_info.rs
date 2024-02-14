use std::str::FromStr;

use crate::{
    jvm::{
        class::ClassReference,
        field::Field,
        parsing::jvm_element_parser::{parse_flags, parse_jvm},
        ClassFileParsingError, ClassFileParsingResult,
    },
    macros::extract_attributes,
    types::field_type::FieldType,
};

use super::{
    jvm_element_parser::ParseJvmElement, parsing_context::ParsingContext, reader_utils::ClassReader,
};

impl<R: std::io::Read> ParseJvmElement<R> for Field {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let access_flags = parse_flags(reader)?;
        let name = parse_jvm!(reader, ctx)?;
        let field_type = parse_jvm!(reader, ctx)?;
        let owner = ClassReference {
            binary_name: ctx.current_class_binary_name.clone(),
        };
        let attributes: Vec<Attribute> = parse_jvm!(u16, reader, ctx)?;
        extract_attributes! {
            for attributes in "field_info" by {
                let constant_value: ConstantValue,
                let signature: Signature,
                let runtime_visible_annotations
                    : RuntimeVisibleAnnotations unwrap_or_default,
                let runtime_invisible_annotations
                    : RuntimeInvisibleAnnotations unwrap_or_default,
                let runtime_visible_type_annotations
                    : RuntimeVisibleTypeAnnotations unwrap_or_default,
                let runtime_invisible_type_annotations
                    : RuntimeInvisibleTypeAnnotations unwrap_or_default,
                if let is_synthetic: Synthetic,
                if let is_deperecated: Deprecated,
            }
        }

        Ok(Field {
            access_flags,
            name,
            owner,
            field_type,
            constant_value,
            is_synthetic,
            is_deperecated,
            signature,
            runtime_visible_annotations,
            runtime_invisible_annotations,
            runtime_visible_type_annotations,
            runtime_invisible_type_annotations,
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for FieldType {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let descriptor_index = reader.read_value()?;
        let descriptor = ctx.constant_pool.get_str(descriptor_index)?;
        FieldType::from_str(descriptor).map_err(ClassFileParsingError::from)
    }
}
