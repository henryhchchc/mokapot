use std::io::{self, Read};

use itertools::Itertools;

use super::{
    FromReader, ParsingContext, ParseError, ToWriter,
    attribute::{Attribute, AttributeInfo},
    errors::GenerationError,
    jvm_element_parser::ClassElement,
    reader_utils::ValueReaderExt,
};
use crate::{
    jvm::{
        Field,
        bytecode::errors::ParsingErrorContext,
        field::{self},
        references::ClassRef,
    },
    macros::{attributes_into_iter, extract_attributes, see_jvm_spec},
    types::Descriptor,
};

/// The raw representation of a `field_info` structure.
#[doc = see_jvm_spec!(4, 5)]
#[derive(Debug)]
pub(crate) struct FieldInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<AttributeInfo>,
}

impl FromReader for FieldInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let access_flags = reader.read_value()?;
        let name_index = reader.read_value()?;
        let descriptor_index = reader.read_value()?;
        let attributes_count: u16 = reader.read_value()?;
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

impl ToWriter for FieldInfo {
    fn to_writer<W: io::Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.access_flags.to_be_bytes())?;
        writer.write_all(&self.name_index.to_be_bytes())?;
        writer.write_all(&self.descriptor_index.to_be_bytes())?;
        self.attributes.to_writer(writer)?;
        Ok(())
    }
}

impl ClassElement for Field {
    type Raw = FieldInfo;

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
        let owner = ClassRef {
            binary_name: ctx.current_class_binary_name.clone(),
        };
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
                else let free_attributes
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
            runtime_visible_annotations,
            runtime_invisible_annotations,
            runtime_visible_type_annotations,
            runtime_invisible_type_annotations,
            free_attributes,
        })
    }

    fn into_raw(
        self,
        cp: &mut crate::jvm::class::ConstantPool,
    ) -> Result<Self::Raw, GenerationError> {
        let access_flags = self.access_flags.into_raw(cp)?;
        let name_index = cp.put_string(self.name)?;
        let descriptor_index = cp.put_string(self.field_type.descriptor())?;
        let attributes = [
            self.constant_value.map(Attribute::ConstantValue),
            self.signature.map(Attribute::Signature),
            self.is_synthetic.then_some(Attribute::Synthetic),
            self.is_deprecated.then_some(Attribute::Deprecated),
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
