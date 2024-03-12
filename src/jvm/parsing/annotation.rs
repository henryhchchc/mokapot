use std::{io::Read, iter::repeat_with};

use crate::jvm::{
    annotation::{Annotation, ElementValue, TargetInfo, TypeAnnotation, TypePathElement},
    code::LocalVariableId,
    field::ConstantValue,
};

use super::{
    jvm_element_parser::{FromRaw, JvmElement},
    raw_attributes,
    reader_utils::ValueReaderExt,
    Context, Error,
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

impl FromRaw for Annotation {
    type Raw = raw_attributes::Annotation;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            type_index,
            element_value_pairs,
        } = raw;
        let annotation_type = ctx.constant_pool.get_str(type_index)?.parse()?;
        let element_value_pairs = element_value_pairs
            .into_iter()
            .map(|(name_idx, raw_value)| {
                let element_name = ctx.constant_pool.get_str(name_idx)?;
                let element_value = ElementValue::from_raw(raw_value, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Annotation {
            annotation_type,
            element_value_pairs,
        })
    }
}
impl JvmElement for TypeAnnotation {
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
        Ok(TypeAnnotation {
            annotation_type,
            target_info,
            target_path,
            element_value_pairs,
        })
    }
}

impl FromRaw for ElementValue {
    type Raw = raw_attributes::ElementValueInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        match raw {
            Self::Raw::ConstValue(b'B' | b'C' | b'I' | b'S' | b'Z', const_value_index) => {
                if let const_value @ ConstantValue::Integer(_) =
                    ctx.constant_pool.get_constant_value(const_value_index)?
                {
                    Ok(Self::Constant(const_value))
                } else {
                    Err(Error::Other("Expecte integer constant value"))
                }
            }
            Self::Raw::ConstValue(b'D', const_value_index) => {
                if let const_value @ ConstantValue::Double(_) =
                    ctx.constant_pool.get_constant_value(const_value_index)?
                {
                    Ok(Self::Constant(const_value))
                } else {
                    Err(Error::Other("Expecte double constant value"))
                }
            }
            Self::Raw::ConstValue(b'F', const_value_index) => {
                if let const_value @ ConstantValue::Float(_) =
                    ctx.constant_pool.get_constant_value(const_value_index)?
                {
                    Ok(Self::Constant(const_value))
                } else {
                    Err(Error::Other("Expecte float constant value"))
                }
            }
            Self::Raw::ConstValue(b'J', const_value_index) => {
                if let const_value @ ConstantValue::Long(_) =
                    ctx.constant_pool.get_constant_value(const_value_index)?
                {
                    Ok(Self::Constant(const_value))
                } else {
                    Err(Error::Other("Expecte long constant value"))
                }
            }
            Self::Raw::ConstValue(b's', const_value_index) => {
                if let const_value @ ConstantValue::String(_) =
                    ctx.constant_pool.get_constant_value(const_value_index)?
                {
                    Ok(Self::Constant(const_value))
                } else {
                    Err(Error::Other("Expecte string constant value"))
                }
            }
            Self::Raw::ConstValue(_, _) => Err(Error::Other("Invalid constant value tag")),
            Self::Raw::EnumConstValue {
                type_name_index,
                const_name_index,
            } => {
                let enum_type = ctx.constant_pool.get_str(type_name_index)?.to_owned();
                let const_name = ctx.constant_pool.get_str(const_name_index)?.to_owned();
                Ok(Self::EnumConstant {
                    enum_type_name: enum_type,
                    const_name,
                })
            }
            Self::Raw::ClassInfo(class_info_index) => {
                let return_descriptor = ctx.constant_pool.get_str(class_info_index)?.parse()?;
                Ok(Self::Class { return_descriptor })
            }
            Self::Raw::AnnotationValue(annotation) => Ok(Self::AnnotationInterface(
                Annotation::from_raw(annotation, ctx)?,
            )),
            Self::Raw::ArrayValue(values) => {
                let values = values
                    .into_iter()
                    .map(|raw_value| FromRaw::from_raw(raw_value, ctx))
                    .collect::<Result<_, _>>()?;
                Ok(Self::Array(values))
            }
        }
    }
}
