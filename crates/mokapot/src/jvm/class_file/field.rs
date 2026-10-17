use crate::jvm::class_file::attribute::extract_attributes;
use crate::jvm::class_file::flags::parse_flags;

use super::{
    Decode, ParseError, ResolutionContext,
    attribute::AttributeInfo,
    decoder::{parse_repeated, read_u16},
    resolve::Resolve,
};
use crate::{
    intrinsics::see_jvm_spec,
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

impl<'a> Decode<'a> for FieldInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let access_flags = read_u16(bytes)?;
        let name_index = read_u16(bytes)?;
        let descriptor_index = read_u16(bytes)?;
        let attributes_count = read_u16(bytes)?;
        let attributes =
            parse_repeated(bytes, usize::from(attributes_count), AttributeInfo::decode)?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl Resolve for FieldInfo<'_> {
    type Output = Field;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let FieldInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        } = raw;
        let access_flags =
            parse_flags::<field::AccessFlags>(access_flags, "Invalid field access flags")?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let field_type = ctx
            .constant_pool
            .get_str(descriptor_index)?
            .parse()
            .context("Invalid field descriptor")?;
        let owner = ctx.current_class_name.clone();
        let attributes = attributes.into_iter().map(|it| it.resolve(ctx));

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
