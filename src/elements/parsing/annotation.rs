use crate::{
    elements::{
        annotation::{Annotation, ElementValue, TargetInfo, TypeAnnotation, TypePathElement},
        class_parser::{ClassFileParsingError, ClassFileParsingResult},
        field::{ConstantValue, FieldType},
    },
    utils::{read_u16, read_u32, read_u8},
};

use super::{attribute::Attribute, constant_pool::ConstantPool};

impl ElementValue {
    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let tag = read_u8(reader)?;

        match tag as char {
            'B' | 'C' | 'I' | 'S' | 'Z' => {
                let const_value_index = read_u16(reader)?;
                let const_value = constant_pool.get_constant_value(&const_value_index)?;
                let ConstantValue::Integer(value) = const_value else {
                    return Err(ClassFileParsingError::MalformedClassFile);
                };
                Ok(Self::Constant(ConstantValue::Integer(value)))
            }
            'D' => {
                let const_value_index = read_u16(reader)?;
                let cons_value = constant_pool.get_constant_value(&const_value_index)?;
                let ConstantValue::Double(value) = cons_value else {
                    return Err(ClassFileParsingError::MalformedClassFile);
                };
                Ok(Self::Constant(ConstantValue::Double(value)))
            }
            'F' => {
                let const_value_index = read_u16(reader)?;
                let const_value = constant_pool.get_constant_value(&const_value_index)?;
                let ConstantValue::Float(value) = const_value else {
                    return Err(ClassFileParsingError::MalformedClassFile);
                };
                Ok(Self::Constant(ConstantValue::Float(value)))
            }
            'J' => {
                let const_value_index = read_u16(reader)?;
                let const_value = constant_pool.get_constant_value(&const_value_index)?;
                let ConstantValue::Long(value) = const_value else {
                    return Err(ClassFileParsingError::MalformedClassFile);
                };
                Ok(Self::Constant(ConstantValue::Long(value)))
            }
            's' => {
                let utf8_idx = read_u16(reader)?;
                let string = constant_pool.get_string(&utf8_idx)?;
                Ok(Self::Constant(ConstantValue::String(string)))
            }
            'e' => {
                let enum_type_name_idx = read_u16(reader)?;
                let enum_type = constant_pool.get_string(&enum_type_name_idx)?;
                let const_name_idx = read_u16(reader)?;
                let const_name = constant_pool.get_string(&const_name_idx)?;
                Ok(Self::EnumConstant {
                    enum_type_name: enum_type,
                    const_name,
                })
            }
            'c' => {
                let class_info_idx = read_u16(reader)?;
                let return_descriptor = constant_pool.get_string(&class_info_idx)?;
                Ok(Self::Class { return_descriptor })
            }
            '@' => Annotation::parse(reader, constant_pool).map(Self::AnnotationInterface),
            '[' => {
                let num_values = read_u16(reader)?;
                let mut values = Vec::with_capacity(num_values as usize);
                for _ in 0..num_values {
                    values.push(Self::parse(reader, constant_pool)?);
                }
                Ok(Self::Array(values))
            }
            _ => Err(ClassFileParsingError::InvalidElementValueTag(tag as char)),
        }
    }
}

impl Annotation {
    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let type_idx = read_u16(reader)?;
        let annotation_type = constant_pool.get_str(&type_idx)?;
        let annotation_type = FieldType::new(annotation_type)?;
        let num_element_value_pairs = read_u16(reader)?;
        let mut element_value_pairs = Vec::with_capacity(num_element_value_pairs as usize);
        for _ in 0..num_element_value_pairs {
            let element_name_idx = read_u16(reader)?;
            let element_name = constant_pool.get_string(&element_name_idx)?;
            let element_value = ElementValue::parse(reader, constant_pool)?;
            element_value_pairs.push((element_name, element_value));
        }
        Ok(Annotation {
            annotation_type,
            element_value_pairs,
        })
    }
}

impl TypePathElement {
    fn parse<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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
    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> ClassFileParsingResult<Self>
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
                let mut table = Vec::with_capacity(table_length as usize);
                for _ in 0..table_length {
                    let start_pc = read_u16(reader)?;
                    let length = read_u16(reader)?;
                    let index = read_u16(reader)?;
                    table.push((start_pc, length, index));
                }
                TargetInfo::LocalVar(table)
            }
            0x42 => TargetInfo::Catch(read_u16(reader)?),
            0x43..=0x46 => TargetInfo::Offset(read_u16(reader)?),
            0x47..=0x4B => TargetInfo::TypeArgument(read_u16(reader)?, read_u8(reader)?),
            _ => Err(ClassFileParsingError::InvalidTargetType(target_type))?,
        };
        let mut target_path = Vec::new();
        let path_length = read_u8(reader)?;
        for _ in 0..path_length {
            let type_path_element = TypePathElement::parse(reader)?;
            target_path.push(type_path_element);
        }
        let type_index = read_u16(reader)?;
        let num_element_value_pairs = read_u16(reader)?;
        let mut element_value_pairs = Vec::with_capacity(num_element_value_pairs as usize);
        for _ in 0..num_element_value_pairs {
            let element_name_idx = read_u16(reader)?;
            let element_name = constant_pool.get_string(&element_name_idx)?;
            let element_value = ElementValue::parse(reader, constant_pool)?;
            element_value_pairs.push((element_name, element_value));
        }
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
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<Annotation>>
    where
        R: std::io::Read,
    {
        // Length is to be checked outside.
        let num_annotations = read_u16(reader)?;
        let mut annotations = Vec::with_capacity(num_annotations as usize);
        for _ in 0..num_annotations {
            let annotation = Annotation::parse(reader, constant_pool)?;
            annotations.push(annotation);
        }

        Ok(annotations)
    }

    pub(super) fn parse_parameter_annotations<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<Vec<Annotation>>>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let num_parameters = read_u8(reader)?;
        let mut parameter_annotations = Vec::with_capacity(num_parameters as usize);
        for _ in 0..num_parameters {
            let par_annotations = Self::parse_annotations(reader, constant_pool)?;
            parameter_annotations.push(par_annotations);
        }
        Ok(parameter_annotations)
    }

    pub(super) fn parse_type_annotations<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<TypeAnnotation>>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let num_annotations = read_u16(reader)?;
        let mut annotations = Vec::with_capacity(num_annotations as usize);
        for _ in 0..num_annotations {
            let type_annotation = TypeAnnotation::parse(reader, constant_pool)?;
            annotations.push(type_annotation);
        }
        Ok(annotations)
    }

    pub(super) fn parse_annotation_default<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let value = ElementValue::parse(reader, constant_pool)?;
        Ok(Self::AnnotationDefault(value))
    }
}
