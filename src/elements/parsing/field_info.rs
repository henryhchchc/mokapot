use crate::{
    elements::field::{Field, FieldAccessFlags},
    errors::ClassFileParsingError,
    reader_utils::read_u16,
    types::FieldType,
};

use super::{
    attribute::{Attribute, AttributeList},
    constant_pool::ParsingContext,
};

impl Field {
    pub(crate) fn parse<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Field, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let access = read_u16(reader)?;
        let Some(access_flags) = FieldAccessFlags::from_bits(access) else {
            return Err(ClassFileParsingError::UnknownFlags(access, "field"));
        };
        let name_index = read_u16(reader)?;
        let name = ctx.get_str(&name_index)?.to_owned();
        let descriptor_index = read_u16(reader)?;
        let descriptor = ctx.get_str(&descriptor_index)?;
        let field_type = FieldType::new(descriptor)?;

        let attributes = AttributeList::parse(reader, ctx)?;
        let mut constant_value = None;
        let mut is_synthetic = false;
        let mut is_deperecated = false;
        let mut signature = None;
        let mut runtime_visible_annotations = None;
        let mut runtime_invisible_annotations = None;
        let mut runtime_visible_type_annotations = None;
        let mut runtime_invisible_type_annotations = None;
        for attr in attributes.into_iter() {
            match attr {
                Attribute::ConstantValue(v) => constant_value = Some(v),
                Attribute::Synthetic => is_synthetic = true,
                Attribute::Deprecated => is_deperecated = true,
                Attribute::Signature(s) => signature = Some(s),
                Attribute::RuntimeVisibleAnnotations(a) => runtime_visible_annotations = Some(a),
                Attribute::RuntimeInvisibleAnnotations(a) => {
                    runtime_invisible_annotations = Some(a)
                }
                Attribute::RuntimeVisibleTypeAnnotations(a) => {
                    runtime_visible_type_annotations = Some(a)
                }
                Attribute::RuntimeInvisibleTypeAnnotations(a) => {
                    runtime_invisible_type_annotations = Some(a)
                }
                it => Err(ClassFileParsingError::UnexpectedAttribute(
                    it.name(),
                    "field_info",
                ))?,
            }
        }

        Ok(Field {
            access_flags,
            name,
            field_type,
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
