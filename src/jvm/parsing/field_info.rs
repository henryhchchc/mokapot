use std::{
    io::{self, Read},
    str::FromStr,
};

use crate::{
    jvm::{
        field::{self, Field},
        references::ClassRef,
    },
    macros::{extract_attributes, see_jvm_spec},
    types::field_type::FieldType,
};

use super::{
    attribute::AttributeInfo,
    jvm_element_parser::{FromRaw, JvmElement},
    reader_utils::{FromReader, ValueReaderExt},
    Context, Error,
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

impl FromRaw for Field {
    type Raw = FieldInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let FieldInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        } = raw;
        let access_flags = field::AccessFlags::from_bits(access_flags)
            .ok_or(Error::UnknownFlags("FieldAccessFlag", access_flags))?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let field_type = ctx.constant_pool.get_str(descriptor_index)?.parse()?;
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
                if let is_deperecated: Deprecated,
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
            is_deperecated,
            signature,
            runtime_visible_annotations,
            runtime_invisible_annotations,
            runtime_visible_type_annotations,
            runtime_invisible_type_annotations,
            free_attributes,
        })
    }
}

impl JvmElement for FieldType {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let descriptor_index = reader.read_value()?;
        let descriptor = ctx.constant_pool.get_str(descriptor_index)?;
        FieldType::from_str(descriptor).map_err(Error::from)
    }
}
