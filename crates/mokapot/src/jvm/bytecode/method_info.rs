use std::io::{self, Read};

use itertools::Itertools;

use super::{
    FromBytecode, ParseError, ToBytecode,
    attribute::{Attribute, AttributeInfo},
    errors::GenerationError,
    jvm_element_parser::ClassElement,
    reader_utils::BytecodeReader,
};
use crate::{
    intrinsics::{attributes_into_iter, extract_attributes, see_jvm_spec},
    jvm::{
        Method,
        bytecode::{ParsingContext, errors::ParsingErrorContext},
        method::{self},
        references::ClassRef,
    },
    types::{Descriptor, method_descriptor::MethodDescriptor},
};

/// The raw representation of a `method_info` structure.
#[doc = see_jvm_spec!(4, 6)]
#[derive(Debug)]
pub(super) struct MethodInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<AttributeInfo>,
}

impl FromBytecode for MethodInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let access_flags = reader.decode_value()?;
        let name_index = reader.decode_value()?;
        let descriptor_index = reader.decode_value()?;
        let attributes_count: u16 = reader.decode_value()?;
        let attributes = (0..attributes_count)
            .map(|_| AttributeInfo::from_reader(reader))
            .collect::<io::Result<_>>()?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl ToBytecode for MethodInfo {
    fn to_writer<W: io::Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.access_flags.to_be_bytes())?;
        writer.write_all(&self.name_index.to_be_bytes())?;
        writer.write_all(&self.descriptor_index.to_be_bytes())?;
        self.attributes.to_writer(writer)?;
        Ok(())
    }
}

impl ClassElement for Method {
    type Raw = MethodInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let MethodInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        } = raw;
        let access_flags = method::AccessFlags::from_bits(access_flags)
            .ok_or(ParseError::malform("Invalid method access flags"))?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let descriptor: MethodDescriptor = ctx
            .constant_pool
            .get_str(descriptor_index)?
            .parse()
            .context("Invalid method descriptor")?;
        let owner = ClassRef {
            binary_name: ctx.current_class_binary_name.clone(),
        };

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
            runtime_visible_annotations,
            runtime_invisible_annotations,
            runtime_visible_type_annotations,
            runtime_invisible_type_annotations,
            runtime_visible_parameter_annotations,
            runtime_invisible_parameter_annotations,
            annotation_default,
            parameters,
            is_synthetic,
            is_deprecated,
            signature,
            other_attributes,
        })
    }

    fn into_raw(
        self,
        cp: &mut crate::jvm::class::ConstantPool,
    ) -> Result<Self::Raw, GenerationError> {
        let access_flags = self.access_flags.into_raw(cp)?;
        let name_index = cp.put_string(self.name)?;
        let descriptor_index = cp.put_string(self.descriptor.descriptor())?;
        let attributes = [
            self.body.map(Attribute::Code),
            Some(self.exceptions)
                .filter(|it| !it.is_empty())
                .map(Attribute::Exceptions),
            Some(self.parameters)
                .filter(|it| !it.is_empty())
                .map(Attribute::MethodParameters),
            self.annotation_default.map(Attribute::AnnotationDefault),
            self.is_synthetic.then_some(Attribute::Synthetic),
            self.is_deprecated.then_some(Attribute::Deprecated),
            self.signature.map(Attribute::Signature),
        ]
        .into_iter()
        .flatten()
        .chain(attributes_into_iter!(self))
        .map(|it| it.into_raw(cp))
        .try_collect()?;

        Ok(Self::Raw {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}
