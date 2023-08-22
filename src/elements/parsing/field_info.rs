use crate::{
    elements::{
        class_parser::{ClassFileParsingError, ClassFileParsingResult},
        field::{Field, FieldAccessFlags, FieldType},
    },
    utils::read_u16,
};

use super::{
    attribute::{Attribute, AttributeList},
    constant_pool::ConstantPool,
};

impl Field {
    pub(crate) fn parse_multiple<R>(
        reader: &mut R,
        fields_count: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<Field>>
    where
        R: std::io::Read,
    {
        let mut fields = Vec::with_capacity(fields_count as usize);
        for _ in 0..fields_count {
            let field = Self::parse(reader, constant_pool)?;
            fields.push(field);
        }
        Ok(fields)
    }

    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> ClassFileParsingResult<Field>
    where
        R: std::io::Read,
    {
        let access = read_u16(reader)?;
        let Some(access_flags) = FieldAccessFlags::from_bits(access) else {
            return Err(ClassFileParsingError::UnknownFlags(access));
        };
        let name_index = read_u16(reader)?;
        let name = constant_pool.get_string(&name_index)?;
        let descriptor_index = read_u16(reader)?;
        let descriptor = constant_pool.get_str(&descriptor_index)?;
        let field_type = FieldType::new(descriptor)?;

        let attributes = AttributeList::parse(reader, constant_pool)?;
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
                    format!("{:?}", it),
                    "field_info".to_string(),
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
