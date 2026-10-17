use zerocopy::byteorder::big_endian::U16;

use super::{
    FromBytecode, ParseError, ParsingContext, attribute::AttributeInfo,
    class_element::ClassElement, decoder::read_value,
};
use crate::{
    intrinsics::{extract_attributes, see_jvm_spec},
    jvm::{
        Annotations, Field,
        errors::ParsingErrorContext,
        field::{self},
    },
};

/// The raw representation of a `field_info` structure.
#[doc = see_jvm_spec!(4, 5)]
#[derive(Debug)]
pub(crate) struct FieldInfo<'a> {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Box<[AttributeInfo<'a>]>,
}

impl<'a> FromBytecode<'a> for FieldInfo<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let access_flags = read_value::<U16>(bytes)?.get();
        let name_index = read_value::<U16>(bytes)?.get();
        let descriptor_index = read_value::<U16>(bytes)?.get();
        let attributes_count = read_value::<U16>(bytes)?.get();
        let attributes = (0..attributes_count)
            .map(|_| AttributeInfo::from_bytes(bytes))
            .collect::<Result<_, ParseError>>()?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl<'a> ClassElement<'a> for Field {
    type Raw = FieldInfo<'a>;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let FieldInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        } = raw;
        let access_flags = field::AccessFlags::from_bits(access_flags)
            .ok_or(ParseError::malform("Invalid field access flags"))?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let field_type = ctx
            .constant_pool
            .get_str(descriptor_index)?
            .parse()
            .context("Invalid field descriptor")?;
        let owner = ctx.current_class_name.clone();
        let attributes: Vec<Attribute> = attributes
            .into_iter()
            .map(|it| Attribute::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;

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
                if let is_deprecated: Deprecated,
                else let other_attributes
            }
        }

        Ok(Field {
            access_flags,
            name,
            owner,
            field_type,
            constant_value,
            is_synthetic,
            is_deprecated,
            signature,
            annotations: Annotations {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: Annotations {
                runtime_visible: runtime_visible_type_annotations,
                runtime_invisible: runtime_invisible_type_annotations,
            },
            other_attributes,
        })
    }
}
