use std::{io::Read, iter::repeat_with};

use crate::jvm::{
    annotation::{Annotation, ElementValue, TargetInfo, Type, TypePathElement},
    code::LocalVariableId,
    field::{ConstantValue, JavaString},
};

use super::{
    jvm_element_parser::JvmElement, Context, reader_utils::ValueReaderExt, Error,
};

impl JvmElement for TypePathElement {
    fn parse<R: Read + ?Sized>(reader: &mut R, _ctx: &Context) -> Result<Self, Error> {
        let kind: u8 = reader.read_value()?;
        let argument_index: u8 = reader.read_value()?;
        match (kind, argument_index) {
            (0, 0) => Ok(Self::Array),
            (1, 0) => Ok(Self::Nested),
            (2, 0) => Ok(Self::Bound),
            (3, idx) => Ok(Self::TypeArgument(idx)),
            _ => Err(Error::InvalidTypePathKind),
        }
    }
}

impl JvmElement for Annotation {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let type_idx = reader.read_value()?;
        let annotation_type = ctx.constant_pool.get_str(type_idx)?.parse()?;
        let num_element_value_pairs: u16 = reader.read_value()?;
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_idx = reader.read_value()?;
                let element_name = ctx.constant_pool.get_str(element_name_idx)?;
                let element_value = ElementValue::parse(reader, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Annotation {
            annotation_type,
            element_value_pairs,
        })
    }
}
impl JvmElement for Type {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let target_type = reader.read_value()?;
        let target_info = match target_type {
            0x00 | 0x01 => TargetInfo::TypeParameter {
                index: reader.read_value()?,
            },
            0x10 => TargetInfo::SuperType {
                index: reader.read_value()?,
            },
            0x11 | 0x12 => TargetInfo::TypeParameterBound {
                type_parameter_index: reader.read_value()?,
                bound_index: reader.read_value()?,
            },
            0x13..=0x15 => TargetInfo::Empty,
            0x16 => TargetInfo::FormalParameter {
                index: reader.read_value()?,
            },
            0x17 => TargetInfo::Throws {
                index: reader.read_value()?,
            },
            0x40 | 0x41 => {
                let table_length: u16 = reader.read_value()?;
                let table = (0..table_length)
                    .map(|_| {
                        let start_pc = reader.read_value::<u16>()?;
                        let length: u16 = reader.read_value()?;
                        let effective_range = start_pc.into()..(start_pc + length).into();
                        let index = reader.read_value()?;
                        Ok(LocalVariableId {
                            effective_range,
                            index,
                        })
                    })
                    .collect::<Result<_, Error>>()?;
                TargetInfo::LocalVar(table)
            }
            0x42 => TargetInfo::Catch {
                index: reader.read_value()?,
            },
            0x43..=0x46 => TargetInfo::Offset(reader.read_value()?),
            0x47..=0x4B => TargetInfo::TypeArgument {
                offset: reader.read_value::<u16>()?.into(),
                index: reader.read_value()?,
            },
            unexpected => Err(Error::InvalidTargetType(unexpected))?,
        };
        // The length of target path is represented by a single byte.
        let target_path_length: u8 = reader.read_value()?;
        let target_path = repeat_with(|| JvmElement::parse(reader, ctx))
            .take(target_path_length.into())
            .collect::<Result<_, Error>>()?;
        let type_index = reader.read_value()?;
        let annotation_type = ctx.constant_pool.get_str(type_index)?.parse()?;
        let num_element_value_pairs: u16 = reader.read_value()?;
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_idx = reader.read_value()?;
                let element_name = ctx.constant_pool.get_str(element_name_idx)?;
                let element_value = ElementValue::parse(reader, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Type {
            annotation_type,
            target_info,
            target_path,
            element_value_pairs,
        })
    }
}

impl JvmElement for ElementValue {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let tag: u8 = reader.read_value()?;

        match tag as char {
            it @ ('B' | 'C' | 'I' | 'S' | 'Z' | 'D' | 'F' | 'J') => {
                let const_value_index = reader.read_value()?;
                let const_value = ctx.constant_pool.get_constant_value(const_value_index)?;
                match (it, &const_value) {
                    ('B' | 'C' | 'I' | 'S' | 'Z', ConstantValue::Integer(_))
                    | ('D', ConstantValue::Double(_))
                    | ('F', ConstantValue::Float(_))
                    | ('J', ConstantValue::Long(_)) => Ok(Self::Constant(const_value)),
                    _ => Err(Error::Other(
                        "Primitive element tag must point to primitive constant values",
                    )),
                }
            }
            's' => {
                let utf8_idx = reader.read_value()?;
                let str = ctx.constant_pool.get_str(utf8_idx)?;
                Ok(Self::Constant(ConstantValue::String(JavaString::Utf8(
                    str.to_owned(),
                ))))
            }
            'e' => {
                let enum_type_name_idx = reader.read_value()?;
                let enum_type = ctx.constant_pool.get_str(enum_type_name_idx)?;
                let const_name_idx = reader.read_value()?;
                let const_name = ctx.constant_pool.get_str(const_name_idx)?.to_owned();
                Ok(Self::EnumConstant {
                    enum_type_name: enum_type.to_owned(),
                    const_name,
                })
            }
            'c' => {
                let class_info_idx = reader.read_value()?;
                let return_descriptor = ctx.constant_pool.get_str(class_info_idx)?.parse()?;
                Ok(Self::Class { return_descriptor })
            }
            '@' => Annotation::parse(reader, ctx).map(Self::AnnotationInterface),
            '[' => {
                let values = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
                Ok(Self::Array(values))
            }
            unexpected => Err(Error::InvalidElementValueTag(unexpected)),
        }
    }
}
