use zerocopy::byteorder::big_endian::U16;

use crate::jvm::class_file::attribute::extract_attributes;
use crate::jvm::class_file::flags::parse_flags;

use super::{
    Decode, ParseError, ResolutionContext,
    attribute::AttributeInfo,
    decoder::{decode_value, parse_repeated},
    resolve::Resolve,
};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        annotation::AnnotationsData,
        field::{self, FieldData},
    },
};

/// The raw representation of a `field_info` structure.
#[doc = see_jvm_spec!(4, 5)]
#[derive(Debug)]
pub(crate) struct FieldInfo<'a> {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Box<[&'a AttributeInfo]>,
}

impl<'a> Decode<'a> for FieldInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let [access_flags, name_index, descriptor_index, attributes_count] =
            decode_value::<[U16; 4]>(bytes)?.map(U16::get);
        let attributes = parse_repeated(bytes, attributes_count, <&AttributeInfo>::decode)?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl Resolve for FieldInfo<'_> {
    type Output = FieldData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        } = self;
        let access_flags =
            parse_flags::<field::AccessFlags>(access_flags, "Invalid field access flags")?;
        let name = ctx.constant_pool.utf8_id(name_index)?;
        let field_type = ctx
            .constant_pool
            .intern_field_type(descriptor_index)
            .map_err(|error| error.add_context("Invalid field descriptor"))?;
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

        Ok(FieldData {
            access_flags,
            name,
            field_type,
            constant_value,
            is_synthetic,
            is_deprecated,
            signature,
            annotations: AnnotationsData {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: AnnotationsData {
                runtime_visible: runtime_visible_type_annotations,
                runtime_invisible: runtime_invisible_type_annotations,
            },
            other_attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::{
        class::Version,
        class_file::{
            class::raw::RecordComponentInfo,
            constant_pool::{RawConstantPool, RawEntry, resolve},
        },
        errors::ParseErrorKind,
    };

    #[test]
    fn descriptor_context_preserves_invalid_pool_index() {
        let pool = RawConstantPool::from_entries([RawEntry::Utf8(b"name")]);
        let mut ctx = ResolutionContext {
            constant_pool: resolve(pool).unwrap(),
            class_version: Version::LATEST,
        };
        let field = FieldInfo {
            access_flags: 0,
            name_index: 1,
            descriptor_index: 2,
            attributes: Box::new([]),
        };
        let component = RecordComponentInfo {
            name_index: 1,
            descriptor_index: 2,
            attributes: Box::new([]),
        };
        let expected = ParseErrorKind::InvalidConstantPoolIndex { index: 2 };
        assert_eq!(field.resolve(&mut ctx).unwrap_err().kind(), expected);
        assert_eq!(component.resolve(&mut ctx).unwrap_err().kind(), expected);
    }
}
