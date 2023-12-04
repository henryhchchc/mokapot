use std::str::FromStr;

use crate::{
    jvm::{
        class::ClassReference,
        field::{Field, FieldAccessFlags},
        ClassFileParsingError, ClassFileParsingResult,
    },
    macros::extract_attributes,
    types::FieldType,
};

use super::{
    parsing_context::ParsingContext,
    reader_utils::{parse_multiple, read_u16},
};

impl Field {
    pub(crate) fn parse<R>(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let access = read_u16(reader)?;
        let Some(access_flags) = FieldAccessFlags::from_bits(access) else {
            return Err(ClassFileParsingError::UnknownFlags(access, "field"));
        };
        let name_index = read_u16(reader)?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let descriptor_index = read_u16(reader)?;
        let descriptor = ctx.constant_pool.get_str(descriptor_index)?;
        let field_type = FieldType::from_str(descriptor)?;
        let owner = ClassReference {
            binary_name: ctx.current_class_binary_name.clone(),
        };

        let attributes = parse_multiple(reader, ctx, Attribute::parse)?;
        extract_attributes! {
            for attributes in "field_info" by {
                let constant_value <= ConstantValue,
                let signature <= Signature,
                let runtime_visible_annotations <= RuntimeVisibleAnnotations,
                let runtime_invisible_annotations <= RuntimeInvisibleAnnotations,
                let runtime_visible_type_annotations <= RuntimeVisibleTypeAnnotations,
                let runtime_invisible_type_annotations <= RuntimeInvisibleTypeAnnotations,
                if Synthetic => is_synthetic = true,
                if Deprecated => is_deperecated = true,
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
            runtime_visible_annotations: runtime_visible_annotations.unwrap_or_default(),
            runtime_invisible_annotations: runtime_invisible_annotations.unwrap_or_default(),
            runtime_visible_type_annotations: runtime_visible_type_annotations.unwrap_or_default(),
            runtime_invisible_type_annotations: runtime_invisible_type_annotations
                .unwrap_or_default(),
        })
    }
}
