use std::str::FromStr;

use crate::{
    jvm::{
        class::ClassReference,
        field::Field,
        parsing::jvm_element_parser::{parse_flags, parse_jvm_element},
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
        let name = parse_jvm_element(reader, ctx)?;
        let field_type = parse_jvm_element(reader, ctx)?;
        let owner = ClassReference {
            binary_name: ctx.current_class_binary_name.clone(),
        };
        let attributes: Vec<Attribute> = parse_jvm_element(reader, ctx)?;
        extract_attributes! {
            for attributes in "field_info" by {
                let constant_value <= ConstantValue,
                let signature <= Signature,
                let runtime_visible_annotations unwrap_or_default <= RuntimeVisibleAnnotations,
                let runtime_invisible_annotations unwrap_or_default <= RuntimeInvisibleAnnotations,
                let runtime_visible_type_annotations unwrap_or_default <= RuntimeVisibleTypeAnnotations,
                let runtime_invisible_type_annotations unwrap_or_default <= RuntimeInvisibleTypeAnnotations,
                if Synthetic => let is_synthetic = true,
                if Deprecated => let is_deperecated = true,
            }
        }

        Ok(Field {
            access_flags,
            name,
            field_type,
            owner,
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
