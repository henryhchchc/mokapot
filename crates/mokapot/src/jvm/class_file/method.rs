use crate::jvm::class_file::attribute::extract_attributes;
use crate::jvm::class_file::flags::parse_flags;

use super::{ParseError, ResolutionContext, class::raw::MemberInfo, resolve::Resolve};
use crate::jvm::{Method, annotation::AnnotationsData, method};

impl MemberInfo<'_> {
    pub(super) fn resolve_method(
        self,
        ctx: &mut ResolutionContext,
    ) -> Result<method::MethodData, ParseError> {
        let Self {
            access_flags,
            name,
            descriptor,
            attributes,
        } = self;
        let access_flags =
            parse_flags::<method::AccessFlags>(access_flags, "Invalid method access flags")?;
        let name = ctx.constant_pool.utf8_id(name)?;
        let descriptor = ctx.constant_pool.intern_method_descriptor(descriptor)?;
        let name_value = ctx.constant_pool.utf8(name);
        let is_initializer = name_value == Method::CLASS_INITIALIZER_NAME;
        let has_parameters = !ctx
            .constant_pool
            .method_descriptor(descriptor)
            .parameters_types
            .is_empty();
        let bytes = name_value.as_bytes();
        if bytes.is_empty()
            || bytes
                .iter()
                .any(|byte| matches!(byte, b'.' | b';' | b'[' | b'/'))
            || (bytes.iter().any(|byte| matches!(byte, b'<' | b'>'))
                && name_value != Method::CONSTRUCTOR_NAME
                && !is_initializer)
        {
            return Err(ParseError::malform("Invalid method name"));
        }

        let attributes = attributes.into_iter().map(|it| it.resolve(ctx));
        extract_attributes! {
            for attributes in "method_info" {
                let body: Code,
                let exceptions: Exceptions as unwrap_or_default,
                let runtime_visible_annotations
                    : RuntimeVisibleAnnotations as unwrap_or_default,
                let runtime_invisible_annotations
                    : RuntimeInvisibleAnnotations as unwrap_or_default,
                let runtime_visible_type_annotations
                    : RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations
                    : RuntimeInvisibleTypeAnnotations as unwrap_or_default,
                let runtime_visible_parameter_annotations
                    : RuntimeVisibleParameterAnnotations as unwrap_or_default,
                let runtime_invisible_parameter_annotations
                    : RuntimeInvisibleParameterAnnotations as unwrap_or_default,
                let annotation_default: AnnotationDefault,
                let parameters: MethodParameters as unwrap_or_default,
                let signature: Signature,
                if let is_synthetic: Synthetic,
                if let is_deprecated: Deprecated,
                else let other_attributes
            }
        };

        // JVM specification 4.7.3
        // If the method is either `native` or `abstract`, and is not a class or interface initialization method
        if (access_flags.contains(method::AccessFlags::NATIVE)
            || access_flags.contains(method::AccessFlags::ABSTRACT))
            && !is_initializer
        {
            // then its method_info structure must not have a Code attribute in its attributes table
            if body.is_some() {
                Err(ParseError::malform("Unexpected code attribute"))?;
            }
        } else {
            // Otherwise, its method_info structure must have exactly one Code attribute in its attributes table
            if body.is_none() {
                Err(ParseError::malform("The method must have a body"))?;
            }
        }

        if ctx.class_version.major() >= 51 && is_initializer {
            // In a class file whose version number is 51.0 or above, the method has its ACC_STATIC flag set and takes no arguments (§4.6).
            if !access_flags.contains(method::AccessFlags::STATIC) || has_parameters {
                Err(ParseError::malform(
                    "Class initializer in class version 51 or above\
                    must be static and takes no arguments",
                ))?;
            }
        }

        Ok(method::MethodData {
            access_flags,
            name,
            descriptor,
            body,
            exceptions,
            annotations: AnnotationsData {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: AnnotationsData {
                runtime_visible: runtime_visible_type_annotations,
                runtime_invisible: runtime_invisible_type_annotations,
            },
            parameter_annotations: AnnotationsData {
                runtime_visible: runtime_visible_parameter_annotations,
                runtime_invisible: runtime_invisible_parameter_annotations,
            },
            annotation_default,
            parameters,
            is_synthetic,
            is_deprecated,
            signature,
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
            Decode,
            attribute::AttributeInfo,
            constant_pool::{RawConstantPool, RawEntry, resolve},
        },
        errors::ParseErrorKind,
    };

    fn class_initializer(
        major: u16,
        is_static: bool,
        descriptor: &[u8],
    ) -> Result<method::MethodData, ParseError> {
        let pool = RawConstantPool::from_entries([
            RawEntry::Utf8(b"<clinit>"),
            RawEntry::Utf8(descriptor),
            RawEntry::Utf8(b"Code"),
        ]);
        let name = pool_index(1);
        let descriptor = pool_index(2);
        let attribute_name_index = 3_u16;
        let mut constant_pool = resolve(pool).unwrap();
        constant_pool.intern_method_descriptor(descriptor)?;
        let mut ctx = ResolutionContext {
            constant_pool,
            class_version: Version::from_versions(major, 0).unwrap(),
        };
        let mut attribute = Vec::new();
        attribute.extend(attribute_name_index.to_be_bytes());
        attribute.extend(13_u32.to_be_bytes());
        attribute.extend([0, 0, 0, 1, 0, 0, 0, 1, 0xb1, 0, 0, 0, 0]);
        let attribute = <&AttributeInfo>::decode(&mut attribute.as_slice()).unwrap();
        MemberInfo {
            access_flags: if is_static {
                method::AccessFlags::STATIC.bits()
            } else {
                0
            },
            name,
            descriptor,
            attributes: Box::new([attribute]),
        }
        .resolve_method(&mut ctx)
    }

    #[test]
    fn class_initializer_constraints_start_at_version_51() {
        for major in [50, 51, 52] {
            for (is_static, descriptor) in
                [(false, b"()V".as_slice()), (true, b"(I)V"), (true, b"()V")]
            {
                let result = class_initializer(major, is_static, descriptor);
                if major < 51 || (is_static && descriptor == b"()V") {
                    result.unwrap();
                } else {
                    assert_eq!(result.unwrap_err().kind(), ParseErrorKind::Malformed);
                }
            }
        }
    }
}
