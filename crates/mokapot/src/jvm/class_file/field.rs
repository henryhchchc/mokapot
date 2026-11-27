use crate::jvm::class_file::attribute::extract_attributes;
use crate::jvm::class_file::flags::parse_flags;

use super::{ParseError, ResolutionContext, class::raw::MemberInfo, resolve::Resolve};
use crate::jvm::{
    annotation::AnnotationsData,
    field::{self, FieldData},
};

impl MemberInfo<'_> {
    pub(super) fn resolve_field(
        self,
        ctx: &mut ResolutionContext,
    ) -> Result<FieldData, ParseError> {
        let Self {
            access_flags,
            name,
            descriptor,
            attributes,
        } = self;
        let access_flags =
            parse_flags::<field::AccessFlags>(access_flags, "Invalid field access flags")?;
        let name = ctx.constant_pool.utf8_id(name)?;
        let field_type = ctx
            .constant_pool
            .intern_field_type(descriptor)
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
    use crate::jvm::constant_pool::pool_index;
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
        let field = MemberInfo {
            access_flags: 0,
            name: pool_index(1),
            descriptor: pool_index(2),
            attributes: Box::new([]),
        };
        let component = RecordComponentInfo {
            name: pool_index(1),
            descriptor: pool_index(2),
            attributes: Box::new([]),
        };
        let expected = ParseErrorKind::InvalidConstantPoolIndex { index: 2 };
        assert_eq!(field.resolve_field(&mut ctx).unwrap_err().kind(), expected);
        assert_eq!(component.resolve(&mut ctx).unwrap_err().kind(), expected);
    }
}
