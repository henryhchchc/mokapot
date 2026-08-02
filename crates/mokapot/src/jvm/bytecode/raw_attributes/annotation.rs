//! Raw attribute types for annotation-related attributes.

use std::io::{self, Read, Write};
use std::result::Result;

use super::super::{
    FromBytecode, ToBytecode, errors::GenerationError, reader_utils::BytecodeReader, write_length,
};
use crate::{intrinsics::enum_discriminant, jvm::code::ProgramCounter};

pub struct Annotation {
    pub type_index: u16,
    pub element_value_pairs: Vec<(u16, ElementValueInfo)>,
}

impl FromBytecode for Annotation {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let type_index = reader.decode_value()?;
        let num_element_value_pairs: u16 = reader.decode_value()?;
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_index = reader.decode_value()?;
                let element_value = reader.decode_value()?;
                Ok((element_name_index, element_value))
            })
            .collect::<io::Result<_>>()?;
        Ok(Self {
            type_index,
            element_value_pairs,
        })
    }
}

impl ToBytecode for Annotation {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.type_index.to_be_bytes())?;
        write_length::<u16>(writer, self.element_value_pairs.len())?;
        for (element_name_index, element_value) in &self.element_value_pairs {
            writer.write_all(&element_name_index.to_be_bytes())?;
            element_value.to_writer(writer)?;
        }
        Ok(())
    }
}

pub enum ElementValueInfo {
    Const(u8, u16),
    Enum {
        type_name_index: u16,
        const_name_index: u16,
    },
    ClassInfo(u16),
    Annotation(Annotation),
    Array(Vec<ElementValueInfo>),
}

impl FromBytecode for ElementValueInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let tag: u8 = reader.decode_value()?;
        match tag {
            tag @ (b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's') => {
                Ok(Self::Const(tag, reader.decode_value()?))
            }
            b'e' => Ok(Self::Enum {
                type_name_index: reader.decode_value()?,
                const_name_index: reader.decode_value()?,
            }),
            b'c' => Ok(Self::ClassInfo(reader.decode_value()?)),
            b'@' => Ok(Self::Annotation(reader.decode_value()?)),
            b'[' => {
                let num_values: u16 = reader.decode_value()?;
                let values = (0..num_values)
                    .map(|_| reader.decode_value())
                    .collect::<io::Result<_>>()?;
                Ok(Self::Array(values))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown element value tag: {tag}"),
            )),
        }
    }
}

impl ToBytecode for ElementValueInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        let tag = self.tag();
        writer.write_all(&tag.to_be_bytes())?;
        match self {
            Self::Const(_, index) | Self::ClassInfo(index) => {
                writer.write_all(&index.to_be_bytes())?;
            }
            Self::Enum {
                type_name_index,
                const_name_index,
            } => {
                writer.write_all(&type_name_index.to_be_bytes())?;
                writer.write_all(&const_name_index.to_be_bytes())?;
            }
            Self::Annotation(annotation) => annotation.to_writer(writer)?,
            Self::Array(values) => {
                write_length::<u16>(writer, values.len())?;
                for value in values {
                    value.to_writer(writer)?;
                }
            }
        }
        Ok(())
    }
}

impl ElementValueInfo {
    const fn tag(&self) -> u8 {
        match self {
            Self::Const(tag, _) => *tag,
            Self::Enum { .. } => b'e',
            Self::ClassInfo { .. } => b'c',
            Self::Annotation(..) => b'@',
            Self::Array(..) => b'[',
        }
    }
}

pub struct TypeAnnotation {
    pub target_info: TargetInfo,
    pub target_path: Vec<(u8, u8)>,
    pub type_index: u16,
    pub element_value_pairs: Vec<(u16, ElementValueInfo)>,
}

impl FromBytecode for TypeAnnotation {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let target_info = reader.decode_value()?;
        let target_path_length: u8 = reader.decode_value()?;
        let target_path = (0..target_path_length)
            .map(|_| {
                let type_path_kind = reader.decode_value()?;
                let type_argument_index = reader.decode_value()?;
                Ok((type_path_kind, type_argument_index))
            })
            .collect::<io::Result<_>>()?;
        let type_index = reader.decode_value()?;
        let num_element_value_pairs: u16 = reader.decode_value()?;
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_index = reader.decode_value()?;
                let element_value = reader.decode_value()?;
                Ok((element_name_index, element_value))
            })
            .collect::<io::Result<_>>()?;
        Ok(Self {
            target_info,
            target_path,
            type_index,
            element_value_pairs,
        })
    }
}

impl ToBytecode for TypeAnnotation {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        self.target_info.to_writer(writer)?;
        write_length::<u8>(writer, self.target_path.len())?;
        for (type_path_kind, type_argument_index) in &self.target_path {
            writer.write_all(&type_path_kind.to_be_bytes())?;
            writer.write_all(&type_argument_index.to_be_bytes())?;
        }
        writer.write_all(&self.type_index.to_be_bytes())?;
        write_length::<u16>(writer, self.element_value_pairs.len())?;
        for (element_name_index, element_value) in &self.element_value_pairs {
            writer.write_all(&element_name_index.to_be_bytes())?;
            element_value.to_writer(writer)?;
        }
        Ok(())
    }
}

#[repr(u8)]
pub enum TargetInfo {
    TypeParameterOfClass {
        index: u8,
    } = 0x00,
    TypeParameterOfMethod {
        index: u8,
    } = 0x01,
    SuperType {
        index: u16,
    } = 0x10,
    TypeParameterBoundOfClass {
        type_parameter_index: u8,
        bound_index: u8,
    } = 0x11,
    TypeParameterBoundOfMethod {
        type_parameter_index: u8,
        bound_index: u8,
    } = 0x12,
    Field = 0x13,
    TypeOfField = 0x14,
    Receiver = 0x15,
    FormalParameter {
        index: u8,
    } = 0x16,
    Throws {
        index: u16,
    } = 0x17,
    LocalVariable(Vec<(ProgramCounter, u16, u16)>) = 0x40,
    ResourceVariable(Vec<(ProgramCounter, u16, u16)>) = 0x41,
    Catch {
        index: u16,
    } = 0x42,
    InstanceOf {
        offset: u16,
    } = 0x43,
    New {
        offset: u16,
    } = 0x44,
    NewMethodReference {
        offset: u16,
    } = 0x45,
    VarMethodReference {
        offset: u16,
    } = 0x46,
    TypeInCast {
        offset: ProgramCounter,
        index: u8,
    } = 0x47,
    TypeArgumentInConstructor {
        offset: ProgramCounter,
        index: u8,
    } = 0x48,
    TypeArgumentInCall {
        offset: ProgramCounter,
        index: u8,
    } = 0x49,
    TypeArgumentInConstructorReference {
        offset: ProgramCounter,
        index: u8,
    } = 0x4A,
    TypeArgumentInMethodReference {
        offset: ProgramCounter,
        index: u8,
    } = 0x4B,
}

impl FromBytecode for TargetInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let target_type: u8 = reader.decode_value()?;
        let target_info = match target_type {
            0x00 => Self::TypeParameterOfClass {
                index: reader.decode_value()?,
            },
            0x01 => Self::TypeParameterOfMethod {
                index: reader.decode_value()?,
            },
            0x10 => Self::SuperType {
                index: reader.decode_value()?,
            },
            0x11 => Self::TypeParameterBoundOfClass {
                type_parameter_index: reader.decode_value()?,
                bound_index: reader.decode_value()?,
            },
            0x12 => Self::TypeParameterBoundOfMethod {
                type_parameter_index: reader.decode_value()?,
                bound_index: reader.decode_value()?,
            },
            0x13 => Self::Field,
            0x14 => Self::TypeOfField,
            0x15 => Self::Receiver,
            0x16 => Self::FormalParameter {
                index: reader.decode_value()?,
            },
            0x17 => Self::Throws {
                index: reader.decode_value()?,
            },
            0x40 => {
                let table_length: u16 = reader.decode_value()?;
                let table = (0..table_length)
                    .map(|_| {
                        let start_pc = reader.decode_value()?;
                        let length = reader.decode_value()?;
                        let index = reader.decode_value()?;
                        Ok((start_pc, length, index))
                    })
                    .collect::<io::Result<_>>()?;
                Self::LocalVariable(table)
            }
            0x41 => {
                let table_length: u16 = reader.decode_value()?;
                let table = (0..table_length)
                    .map(|_| {
                        let start_pc = reader.decode_value()?;
                        let length = reader.decode_value()?;
                        let index = reader.decode_value()?;
                        Ok((start_pc, length, index))
                    })
                    .collect::<io::Result<_>>()?;
                Self::ResourceVariable(table)
            }
            0x42 => Self::Catch {
                index: reader.decode_value()?,
            },
            0x43 => Self::InstanceOf {
                offset: reader.decode_value()?,
            },
            0x44 => Self::New {
                offset: reader.decode_value()?,
            },
            0x45 => Self::NewMethodReference {
                offset: reader.decode_value()?,
            },
            0x46 => Self::VarMethodReference {
                offset: reader.decode_value()?,
            },
            0x47 => Self::TypeInCast {
                offset: reader.decode_value()?,
                index: reader.decode_value()?,
            },
            0x48 => Self::TypeArgumentInConstructor {
                offset: reader.decode_value()?,
                index: reader.decode_value()?,
            },
            0x49 => Self::TypeArgumentInCall {
                offset: reader.decode_value()?,
                index: reader.decode_value()?,
            },
            0x4a => Self::TypeArgumentInConstructorReference {
                offset: reader.decode_value()?,
                index: reader.decode_value()?,
            },
            0x4b => Self::TypeArgumentInMethodReference {
                offset: reader.decode_value()?,
                index: reader.decode_value()?,
            },
            unexpected => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid target type: {unexpected:x}"),
            ))?,
        };
        Ok(target_info)
    }
}

impl ToBytecode for TargetInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        // Safety: Self is marked as repr(u8) so it is fine to use enum_discriminant
        let target_type: u8 = unsafe { enum_discriminant(self) };
        writer.write_all(&target_type.to_be_bytes())?;
        match self {
            TargetInfo::TypeParameterOfClass { index }
            | TargetInfo::TypeParameterOfMethod { index }
            | TargetInfo::FormalParameter { index } => {
                writer.write_all(&index.to_be_bytes())?;
            }
            TargetInfo::TypeParameterBoundOfClass {
                type_parameter_index,
                bound_index,
            }
            | TargetInfo::TypeParameterBoundOfMethod {
                type_parameter_index,
                bound_index,
            } => {
                writer.write_all(&type_parameter_index.to_be_bytes())?;
                writer.write_all(&bound_index.to_be_bytes())?;
            }
            TargetInfo::Field | TargetInfo::TypeOfField | TargetInfo::Receiver => {}
            TargetInfo::LocalVariable(entries) | TargetInfo::ResourceVariable(entries) => {
                write_length::<u16>(writer, entries.len())?;
                for &(start_pc, length, index) in entries {
                    start_pc.to_writer(writer)?;
                    writer.write_all(&length.to_be_bytes())?;
                    writer.write_all(&index.to_be_bytes())?;
                }
            }
            TargetInfo::SuperType { index: value }
            | TargetInfo::Throws { index: value }
            | TargetInfo::Catch { index: value }
            | TargetInfo::InstanceOf { offset: value }
            | TargetInfo::New { offset: value }
            | TargetInfo::NewMethodReference { offset: value }
            | TargetInfo::VarMethodReference { offset: value } => {
                writer.write_all(&value.to_be_bytes())?;
            }
            TargetInfo::TypeInCast { offset, index }
            | TargetInfo::TypeArgumentInConstructor { offset, index }
            | TargetInfo::TypeArgumentInCall { offset, index }
            | TargetInfo::TypeArgumentInConstructorReference { offset, index }
            | TargetInfo::TypeArgumentInMethodReference { offset, index } => {
                offset.to_writer(writer)?;
                writer.write_all(&index.to_be_bytes())?;
            }
        }
        Ok(())
    }
}
