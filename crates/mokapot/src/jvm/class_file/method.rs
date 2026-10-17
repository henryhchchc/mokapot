use super::fixed_layout::FixedLayout;
use crate::jvm::class_file::attribute::extract_attributes;
use crate::jvm::class_file::flags::parse_flags;
use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::{
    Decode, ParseError,
    attribute::AttributeInfo,
    decoder::{decode_value, parse_repeated},
    resolve::Resolve,
};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        Annotations, Method,
        class_file::ResolutionContext,
        errors::ParsingErrorContext,
        method::{self, ParameterAccessFlags, ParameterInfo},
    },
    types::method_descriptor::MethodDescriptor,
};

/// The raw representation of a `method_info` structure.
#[doc = see_jvm_spec!(4, 6)]
#[derive(Debug)]
pub(super) struct MethodInfo<'a> {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Box<[&'a AttributeInfo]>,
}

impl<'a> Decode<'a> for MethodInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let access_flags = decode_value(bytes)?;
        let name_index = decode_value(bytes)?;
        let descriptor_index = decode_value(bytes)?;
        let attributes_count: u16 = decode_value(bytes)?;
        let attributes = parse_repeated(bytes, attributes_count, <&AttributeInfo>::decode)?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl Resolve for MethodInfo<'_> {
    type Output = Method;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let MethodInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        } = raw;
        let access_flags =
            parse_flags::<method::AccessFlags>(access_flags, "Invalid method access flags")?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let descriptor: MethodDescriptor = ctx
            .constant_pool
            .get_str(descriptor_index)?
            .parse()
            .context("Invalid method descriptor")?;
        let owner = ctx.current_class_name.clone();

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
            && name != Method::CLASS_INITIALIZER_NAME
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

        if ctx.class_version.major() >= 51 && name == Method::CLASS_INITIALIZER_NAME {
            // In a class file whose version number is 51.0 or above, the method has its ACC_STATIC flag set and takes no arguments (§4.6).
            if !access_flags.contains(method::AccessFlags::STATIC)
                || !descriptor.parameters_types.is_empty()
            {
                Err(ParseError::malform(
                    "Class initializer in class version 51 or above\
                    must be static and takes no arguments",
                ))?;
            }
        }

        Ok(Method {
            access_flags,
            name,
            descriptor,
            owner,
            body,
            exceptions,
            annotations: Annotations {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: Annotations {
                runtime_visible: runtime_visible_type_annotations,
                runtime_invisible: runtime_invisible_type_annotations,
            },
            parameter_annotations: Annotations {
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

#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub(super) struct MethodParameterInfo {
    pub name_index: U16,
    pub access_flags: U16,
}

impl FixedLayout for MethodParameterInfo {}

impl Resolve for MethodParameterInfo {
    type Output = ParameterInfo;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let MethodParameterInfo {
            name_index,
            access_flags,
        } = raw;
        let name_index = name_index.get();
        let access_flags = access_flags.get();
        let name = if name_index == 0 {
            None
        } else {
            Some(ctx.constant_pool.get_str(name_index)?.to_owned())
        };
        let access_flags =
            parse_flags::<ParameterAccessFlags>(access_flags, "Invalid parameter access flags")?;
        Ok(ParameterInfo { name, access_flags })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::{
        class::Version,
        class_file::constant_pool::{ConstantPool, RawConstantPool, RawEntry},
        errors::ParseErrorKind,
    };

    fn class_initializer(
        major: u16,
        is_static: bool,
        descriptor: &[u8],
    ) -> Result<Method, ParseError> {
        let mut pool = RawConstantPool::new();
        let name_index = pool.put_entry(RawEntry::Utf8(b"<clinit>")).unwrap();
        let descriptor_index = pool.put_entry(RawEntry::Utf8(descriptor)).unwrap();
        let attribute_name_index = pool.put_entry(RawEntry::Utf8(b"Code")).unwrap();
        let ctx = ResolutionContext {
            constant_pool: ConstantPool::from_raw(pool).unwrap(),
            class_version: Version::from_versions(major, 0).unwrap(),
            current_class_name: "Test".parse().unwrap(),
        };
        let mut attribute = Vec::new();
        attribute.extend(attribute_name_index.to_be_bytes());
        attribute.extend(13_u32.to_be_bytes());
        attribute.extend([0, 0, 0, 1, 0, 0, 0, 1, 0xb1, 0, 0, 0, 0]);
        let attribute = <&AttributeInfo>::decode(&mut attribute.as_slice()).unwrap();
        MethodInfo {
            access_flags: if is_static {
                method::AccessFlags::STATIC.bits()
            } else {
                0
            },
            name_index,
            descriptor_index,
            attributes: Box::new([attribute]),
        }
        .resolve(&ctx)
    }

    #[test]
    fn class_initializer_constraints_start_at_version_51() {
        for major in [50, 51, 52] {
            for (is_static, descriptor) in
                [(false, b"()V".as_slice()), (true, b"(I)V"), (true, b"()V")]
            {
                let result = class_initializer(major, is_static, descriptor);
                if major < 51 || (is_static && descriptor == b"()V") {
                    let method = result.unwrap();
                    assert_eq!(method.name, Method::CLASS_INITIALIZER_NAME);
                } else {
                    assert_eq!(result.unwrap_err().kind(), ParseErrorKind::Malformed);
                }
            }
        }
    }
}
