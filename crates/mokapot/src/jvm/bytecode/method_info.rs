use crate::jvm::bytecode::flags::parse_flags;

use super::{
    FromBytecode, ParseError,
    attribute::AttributeInfo,
    decoder::{parse_repeated, read_u16},
    from_raw::FromRaw,
};
use crate::{
    intrinsics::{extract_attributes, see_jvm_spec},
    jvm::{
        Annotations, Method,
        bytecode::ParsingContext,
        errors::ParsingErrorContext,
        method::{self},
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
    attributes: Box<[AttributeInfo<'a>]>,
}

impl<'a> FromBytecode<'a> for MethodInfo<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let access_flags = read_u16(bytes)?;
        let name_index = read_u16(bytes)?;
        let descriptor_index = read_u16(bytes)?;
        let attributes_count: u16 = read_u16(bytes)?;
        let attributes = parse_repeated(
            bytes,
            usize::from(attributes_count),
            AttributeInfo::from_bytes,
        )?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl<'a> FromRaw<'a> for Method {
    type Raw = MethodInfo<'a>;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
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

        let attributes: Vec<Attribute> = attributes
            .into_iter()
            .map(|it| Attribute::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;
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

        if ctx.class_version.major() > 51 && name == Method::CLASS_INITIALIZER_NAME {
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
