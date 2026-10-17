//! Raw attribute types for annotation-related attributes.

use zerocopy::byteorder::big_endian::U16;

use super::super::{
    FromBytecode, ParseError,
    decoder::{BytecodeSlice, read_value},
};
use crate::{intrinsics::see_jvm_spec, jvm::code::ProgramCounter};

#[doc = see_jvm_spec!(4, 7, 16)]
pub struct Annotation {
    pub type_index: u16,
    pub element_value_pairs: Box<[(u16, ElementValueInfo)]>,
}

impl FromBytecode for Annotation {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let type_index = read_value::<U16>(bytes)?.get();
        let num_element_value_pairs: u16 = read_value::<U16>(bytes)?.get();
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_index = read_value::<U16>(bytes)?.get();
                let element_value = bytes.decode_value()?;
                Ok((element_name_index, element_value))
            })
            .collect::<Result<_, ParseError>>()?;
        Ok(Self {
            type_index,
            element_value_pairs,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 16, 1)]
pub enum ElementValueInfo {
    Const(u8, u16),
    Enum {
        type_name_index: u16,
        const_name_index: u16,
    },
    ClassInfo(u16),
    Annotation(Annotation),
    Array(Box<[ElementValueInfo]>),
}

impl FromBytecode for ElementValueInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let tag: u8 = read_value::<u8>(bytes)?;
        match tag {
            tag @ (b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's') => {
                Ok(Self::Const(tag, read_value::<U16>(bytes)?.get()))
            }
            b'e' => Ok(Self::Enum {
                type_name_index: read_value::<U16>(bytes)?.get(),
                const_name_index: read_value::<U16>(bytes)?.get(),
            }),
            b'c' => Ok(Self::ClassInfo(read_value::<U16>(bytes)?.get())),
            b'@' => Ok(Self::Annotation(Annotation::from_bytes(bytes)?)),
            b'[' => {
                let num_values: u16 = read_value::<U16>(bytes)?.get();
                let values = (0..num_values)
                    .map(|_| Self::from_bytes(bytes))
                    .collect::<Result<_, ParseError>>()?;
                Ok(Self::Array(values))
            }
            _ => Err(ParseError::malform(format!(
                "Unknown element value tag: {tag}"
            ))),
        }
    }
}

#[doc = see_jvm_spec!(4, 7, 20)]
pub struct TypeAnnotation {
    pub target_info: TargetInfo,
    pub target_path: Box<[(u8, u8)]>,
    pub type_index: u16,
    pub element_value_pairs: Box<[(u16, ElementValueInfo)]>,
}

impl FromBytecode for TypeAnnotation {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let target_info = bytes.decode_value()?;
        let target_path_length: u8 = read_value::<u8>(bytes)?;
        let target_path = (0..target_path_length)
            .map(|_| {
                let type_path_kind = read_value::<u8>(bytes)?;
                let type_argument_index = read_value::<u8>(bytes)?;
                Ok((type_path_kind, type_argument_index))
            })
            .collect::<Result<_, ParseError>>()?;
        let type_index = read_value::<U16>(bytes)?.get();
        let num_element_value_pairs: u16 = read_value::<U16>(bytes)?.get();
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_index = read_value::<U16>(bytes)?.get();
                let element_value = bytes.decode_value()?;
                Ok((element_name_index, element_value))
            })
            .collect::<Result<_, ParseError>>()?;
        Ok(Self {
            target_info,
            target_path,
            type_index,
            element_value_pairs,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 20, 1)]
#[repr(u8)]
pub enum TargetInfo {
    TypeParameterOfClass {
        type_parameter_index: u8,
    } = 0x00,
    TypeParameterOfMethod {
        type_parameter_index: u8,
    } = 0x01,
    SuperType {
        supertype_index: u16,
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
    MethodReturn = 0x14,
    Receiver = 0x15,
    FormalParameter {
        formal_parameter_index: u8,
    } = 0x16,
    Throws {
        throws_type_index: u16,
    } = 0x17,
    LocalVariable(Box<[(ProgramCounter, u16, u16)]>) = 0x40,
    ResourceVariable(Box<[(ProgramCounter, u16, u16)]>) = 0x41,
    Catch {
        exception_table_index: u16,
    } = 0x42,
    InstanceOf {
        offset: u16,
    } = 0x43,
    New {
        offset: u16,
    } = 0x44,
    ConstructorReference {
        offset: u16,
    } = 0x45,
    MethodReference {
        offset: u16,
    } = 0x46,
    TypeInCast {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x47,
    TypeArgumentInConstructor {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x48,
    TypeArgumentInCall {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x49,
    TypeArgumentInConstructorReference {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x4A,
    TypeArgumentInMethodReference {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x4B,
}

impl FromBytecode for TargetInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let target_type: u8 = read_value::<u8>(bytes)?;
        let target_info = match target_type {
            0x00 => Self::TypeParameterOfClass {
                type_parameter_index: read_value::<u8>(bytes)?,
            },
            0x01 => Self::TypeParameterOfMethod {
                type_parameter_index: read_value::<u8>(bytes)?,
            },
            0x10 => Self::SuperType {
                supertype_index: read_value::<U16>(bytes)?.get(),
            },
            0x11 => Self::TypeParameterBoundOfClass {
                type_parameter_index: read_value::<u8>(bytes)?,
                bound_index: read_value::<u8>(bytes)?,
            },
            0x12 => Self::TypeParameterBoundOfMethod {
                type_parameter_index: read_value::<u8>(bytes)?,
                bound_index: read_value::<u8>(bytes)?,
            },
            0x13 => Self::Field,
            0x14 => Self::MethodReturn,
            0x15 => Self::Receiver,
            0x16 => Self::FormalParameter {
                formal_parameter_index: read_value::<u8>(bytes)?,
            },
            0x17 => Self::Throws {
                throws_type_index: read_value::<U16>(bytes)?.get(),
            },
            0x40 => {
                let table_length: u16 = read_value::<U16>(bytes)?.get();
                let table = (0..table_length)
                    .map(|_| {
                        let start_pc = bytes.decode_value()?;
                        let length = read_value::<U16>(bytes)?.get();
                        let index = read_value::<U16>(bytes)?.get();
                        Ok((start_pc, length, index))
                    })
                    .collect::<Result<_, ParseError>>()?;
                Self::LocalVariable(table)
            }
            0x41 => {
                let table_length: u16 = read_value::<U16>(bytes)?.get();
                let table = (0..table_length)
                    .map(|_| {
                        let start_pc = bytes.decode_value()?;
                        let length = read_value::<U16>(bytes)?.get();
                        let index = read_value::<U16>(bytes)?.get();
                        Ok((start_pc, length, index))
                    })
                    .collect::<Result<_, ParseError>>()?;
                Self::ResourceVariable(table)
            }
            0x42 => Self::Catch {
                exception_table_index: read_value::<U16>(bytes)?.get(),
            },
            0x43 => Self::InstanceOf {
                offset: read_value::<U16>(bytes)?.get(),
            },
            0x44 => Self::New {
                offset: read_value::<U16>(bytes)?.get(),
            },
            0x45 => Self::ConstructorReference {
                offset: read_value::<U16>(bytes)?.get(),
            },
            0x46 => Self::MethodReference {
                offset: read_value::<U16>(bytes)?.get(),
            },
            0x47 => Self::TypeInCast {
                offset: bytes.decode_value()?,
                type_argument_index: read_value::<u8>(bytes)?,
            },
            0x48 => Self::TypeArgumentInConstructor {
                offset: bytes.decode_value()?,
                type_argument_index: read_value::<u8>(bytes)?,
            },
            0x49 => Self::TypeArgumentInCall {
                offset: bytes.decode_value()?,
                type_argument_index: read_value::<u8>(bytes)?,
            },
            0x4a => Self::TypeArgumentInConstructorReference {
                offset: bytes.decode_value()?,
                type_argument_index: read_value::<u8>(bytes)?,
            },
            0x4b => Self::TypeArgumentInMethodReference {
                offset: bytes.decode_value()?,
                type_argument_index: read_value::<u8>(bytes)?,
            },
            unexpected => Err(ParseError::malform(format!(
                "Invalid target type: {unexpected:x}"
            )))?,
        };
        Ok(target_info)
    }
}
