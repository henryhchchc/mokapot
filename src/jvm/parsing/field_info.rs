use std::{io::Read, str::FromStr};

use crate::{
    jvm::{class::ClassReference, field::Field, parsing::jvm_element_parser::parse_flags},
    macros::extract_attributes,
    types::field_type::FieldType,
};

use super::{
    jvm_element_parser::JvmElement, parsing_context::ParsingContext, reader_utils::ValueReaderExt,
    Error,
};

impl JvmElement for Field {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let access_flags = parse_flags(reader)?;
        let name = JvmElement::parse(reader, ctx)?;
        let field_type = JvmElement::parse(reader, ctx)?;
        let owner = ClassReference {
            binary_name: ctx.current_class_binary_name.clone(),
        };
        let attributes: Vec<Attribute> = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
        extract_attributes! {
            for attributes in "field_info" {
                let constant_value: ConstantValue,
                let signature: Signature,
                let runtime_visible_annotations
                    : RuntimeVisibleAnnotations as unwrap_or_default,
                let runtime_invisible_annotations
                    : RuntimeInvisibleAnnotations as unwrap_or_default,
                let runtime_visible_type_annotations
                    : RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations
                    : RuntimeInvisibleTypeAnnotations as unwrap_or_default,
                if let is_synthetic: Synthetic,
                if let is_deperecated: Deprecated,
                else let free_attributes
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
            free_attributes,
        })
    }
}

impl JvmElement for FieldType {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let descriptor_index = reader.read_value()?;
        let descriptor = ctx.constant_pool.get_str(descriptor_index)?;
        FieldType::from_str(descriptor).map_err(Error::from)
    }
}
