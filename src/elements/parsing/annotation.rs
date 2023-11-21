use std::str::FromStr;

use crate::{
    elements::{
        annotation::{Annotation, ElementValue, TargetInfo, TypeAnnotation, TypePathElement},
        field::ConstantValue,
        JavaString,
    },
    errors::ClassFileParsingError,
    reader_utils::{read_u16, read_u32, read_u8},
    types::FieldType,
};

use super::{attribute::Attribute, parsing_context::ParsingContext};

impl ElementValue {
    fn parse<R>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let tag = read_u8(reader)? as char;

        match tag {
            'B' | 'C' | 'I' | 'S' | 'Z' | 'D' | 'F' | 'J' => {
                let const_value_index = read_u16(reader)?;
                let const_value = ctx.constant_pool.get_constant_value(const_value_index)?;
                match (tag, &const_value) {
                    ('B' | 'C' | 'I' | 'S' | 'Z', ConstantValue::Integer(_))
                    | ('D', ConstantValue::Double(_))
                    | ('F', ConstantValue::Float(_))
                    | ('J', ConstantValue::Long(_)) => Ok(Self::Constant(const_value)),
                    _ => Err(ClassFileParsingError::MalformedClassFile(
                        "Primitive element tag must point to primitive constant values",
                    )),
                }
            }
            's' => {
                let utf8_idx = read_u16(reader)?;
                let str = ctx.constant_pool.get_str(utf8_idx)?;
                Ok(Self::Constant(ConstantValue::String(
                    JavaString::ValidUtf8(str.to_owned()),
                )))
            }
            'e' => {
                let enum_type_name_idx = read_u16(reader)?;
                let enum_type = ctx.constant_pool.get_str(enum_type_name_idx)?;
                let const_name_idx = read_u16(reader)?;
                let const_name = ctx.constant_pool.get_str(const_name_idx)?.to_owned();
                Ok(Self::EnumConstant {
                    enum_type_name: enum_type.to_owned(),
                    const_name,
                })
            }
            'c' => {
                let class_info_idx = read_u16(reader)?;
                let return_descriptor = ctx.constant_pool.get_str(class_info_idx)?.to_owned();
                Ok(Self::Class { return_descriptor })
            }
            '@' => Annotation::parse(reader, ctx).map(Self::AnnotationInterface),
            '[' => {
                let num_values = read_u16(reader)?;
                let values = (0..num_values)
                    .map(|_| Self::parse(reader, ctx))
                    .collect::<Result<_, ClassFileParsingError>>()?;
                Ok(Self::Array(values))
            }
            _ => Err(ClassFileParsingError::InvalidElementValueTag(tag as char)),
        }
    }
}

impl Annotation {
    fn parse<R>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let type_idx = read_u16(reader)?;
        let annotation_type = ctx.constant_pool.get_str(type_idx)?;
        let annotation_type = FieldType::from_str(annotation_type)?;
        let num_element_value_pairs = read_u16(reader)?;
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_idx = read_u16(reader)?;
                let element_name = ctx.constant_pool.get_str(element_name_idx)?;
                let element_value = ElementValue::parse(reader, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, ClassFileParsingError>>()?;
        Ok(Annotation {
            annotation_type,
            element_value_pairs,
        })
    }
}

impl TypePathElement {
    fn parse<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let kind = read_u8(reader)?;
        let argument_index = read_u8(reader)?;
        match (kind, argument_index) {
            (0, 0) => Ok(Self::Array),
            (1, 0) => Ok(Self::Nested),
            (2, 0) => Ok(Self::Bound),
            (3, idx) => Ok(Self::TypeArgument(idx)),
            _ => Err(ClassFileParsingError::InvalidTypePathKind),
        }
    }
}

impl TypeAnnotation {
    fn parse<R>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let target_type = read_u8(reader)?;
        let target_info = match target_type {
            0x00 | 0x01 => TargetInfo::TypeParameter(read_u8(reader)?),
            0x10 => TargetInfo::SuperType(read_u16(reader)?),
            0x11 | 0x12 => TargetInfo::TypeParameterBound(read_u8(reader)?, read_u8(reader)?),
            0x13..=0x15 => TargetInfo::Empty,
            0x16 => TargetInfo::FormalParameter(read_u8(reader)?),
            0x17 => TargetInfo::Throws(read_u16(reader)?),
            0x40 | 0x41 => {
                let table_length = read_u16(reader)?;
                let table = (0..table_length)
                    .map(|_| {
                        let start_pc = read_u16(reader)?;
                        let length = read_u16(reader)?;
                        let index = read_u16(reader)?;
                        Ok((start_pc, length, index))
                    })
                    .collect::<Result<_, ClassFileParsingError>>()?;
                TargetInfo::LocalVar(table)
            }
            0x42 => TargetInfo::Catch(read_u16(reader)?),
            0x43..=0x46 => TargetInfo::Offset(read_u16(reader)?),
            0x47..=0x4B => TargetInfo::TypeArgument(read_u16(reader)?, read_u8(reader)?),
            _ => Err(ClassFileParsingError::InvalidTargetType(target_type))?,
        };
        let path_length = read_u8(reader)?;
        let target_path = (0..path_length)
            .map(|_| TypePathElement::parse(reader))
            .collect::<Result<_, _>>()?;
        let type_index = read_u16(reader)?;
        let num_element_value_pairs = read_u16(reader)?;
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_idx = read_u16(reader)?;
                let element_name = ctx.constant_pool.get_str(element_name_idx)?;
                let element_value = ElementValue::parse(reader, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, ClassFileParsingError>>()?;
        Ok(TypeAnnotation {
            target_info,
            target_path,
            type_index,
            element_value_pairs,
        })
    }
}

impl Attribute {
    pub(super) fn parse_annotations<R>(
        reader: &mut R,
        ctx: &ParsingContext,
        _attribute_length: Option<u32>,
    ) -> Result<Vec<Annotation>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        // Attribute length is to be checked outside.
        let num_annotations = read_u16(reader)?;
        let annotations = (0..num_annotations)
            .map(|_| Annotation::parse(reader, ctx))
            .collect::<Result<_, _>>()?;
        Ok(annotations)
    }

    pub(super) fn parse_parameter_annotations<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Vec<Vec<Annotation>>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let num_parameters = read_u8(reader)?;
        let parameter_annotations = (0..num_parameters)
            .map(|_| Self::parse_annotations(reader, ctx, None))
            .collect::<Result<_, _>>()?;
        Ok(parameter_annotations)
    }

    pub(super) fn parse_type_annotations<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Vec<TypeAnnotation>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let num_annotations = read_u16(reader)?;
        let annotations = (0..num_annotations)
            .map(|_| TypeAnnotation::parse(reader, ctx))
            .collect::<Result<_, _>>()?;
        Ok(annotations)
    }

    pub(super) fn parse_annotation_default<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let value = ElementValue::parse(reader, ctx)?;
        Ok(Self::AnnotationDefault(value))
    }
}
