//! Raw attribute types for annotation-related attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::{
    Decode, ParseError,
    decoder::{borrow_slice, decode_value, parse_repeated},
    fixed_layout::FixedLayout,
};
use crate::intrinsics::see_jvm_spec;

#[doc = see_jvm_spec!(4, 7, 16)]
pub struct Annotation {
    pub type_index: u16,
    pub element_value_pairs: Box<[(u16, ElementValueInfo)]>,
}

impl<'a> Decode<'a> for Annotation {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let type_index = decode_value(bytes)?;
        let num_element_value_pairs: u16 = decode_value(bytes)?;
        let element_value_pairs = parse_repeated(bytes, num_element_value_pairs, |bytes| {
            let element_name_index = decode_value(bytes)?;
            let element_value = ElementValueInfo::decode(bytes)?;
            Ok((element_name_index, element_value))
        })?;
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

impl<'a> Decode<'a> for ElementValueInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let tag: u8 = decode_value(bytes)?;
        match tag {
            tag @ (b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's') => {
                Ok(Self::Const(tag, decode_value(bytes)?))
            }
            b'e' => Ok(Self::Enum {
                type_name_index: decode_value(bytes)?,
                const_name_index: decode_value(bytes)?,
            }),
            b'c' => Ok(Self::ClassInfo(decode_value(bytes)?)),
            b'@' => Ok(Self::Annotation(Annotation::decode(bytes)?)),
            b'[' => {
                let num_values: u16 = decode_value(bytes)?;
                let values = parse_repeated(bytes, num_values, Self::decode)?;
                Ok(Self::Array(values))
            }
            _ => Err(ParseError::malform(format!(
                "Unknown element value tag: {tag}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct TypePathElement {
    pub kind: u8,
    pub argument_index: u8,
}

impl FixedLayout for TypePathElement {}

#[doc = see_jvm_spec!(4, 7, 20)]
pub struct TypeAnnotation<'a> {
    pub target_info: TargetInfo<'a>,
    pub target_path: &'a [TypePathElement],
    pub type_index: u16,
    pub element_value_pairs: Box<[(u16, ElementValueInfo)]>,
}

impl<'a> Decode<'a> for TypeAnnotation<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let target_info = TargetInfo::decode(bytes)?;
        let target_path_length: u8 = decode_value(bytes)?;
        let target_path = borrow_slice(bytes, usize::from(target_path_length))?;
        let type_index = decode_value(bytes)?;
        let num_element_value_pairs: u16 = decode_value(bytes)?;
        let element_value_pairs = parse_repeated(bytes, num_element_value_pairs, |bytes| {
            let element_name_index = decode_value(bytes)?;
            let element_value = ElementValueInfo::decode(bytes)?;
            Ok((element_name_index, element_value))
        })?;
        Ok(Self {
            target_info,
            target_path,
            type_index,
            element_value_pairs,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 20, 1)]
#[derive(Debug)]
#[repr(u8)]
pub enum TargetInfo<'a> {
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
    LocalVariable(&'a [LocalVariableTarget]) = 0x40,
    ResourceVariable(&'a [LocalVariableTarget]) = 0x41,
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
        offset: u16,
        type_argument_index: u8,
    } = 0x47,
    TypeArgumentInConstructor {
        offset: u16,
        type_argument_index: u8,
    } = 0x48,
    TypeArgumentInCall {
        offset: u16,
        type_argument_index: u8,
    } = 0x49,
    TypeArgumentInConstructorReference {
        offset: u16,
        type_argument_index: u8,
    } = 0x4A,
    TypeArgumentInMethodReference {
        offset: u16,
        type_argument_index: u8,
    } = 0x4B,
}

impl<'a> Decode<'a> for TargetInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let target_type: u8 = decode_value(bytes)?;
        let target_info = match target_type {
            0x00 => Self::TypeParameterOfClass {
                type_parameter_index: decode_value(bytes)?,
            },
            0x01 => Self::TypeParameterOfMethod {
                type_parameter_index: decode_value(bytes)?,
            },
            0x10 => Self::SuperType {
                supertype_index: decode_value(bytes)?,
            },
            0x11 => Self::TypeParameterBoundOfClass {
                type_parameter_index: decode_value(bytes)?,
                bound_index: decode_value(bytes)?,
            },
            0x12 => Self::TypeParameterBoundOfMethod {
                type_parameter_index: decode_value(bytes)?,
                bound_index: decode_value(bytes)?,
            },
            0x13 => Self::Field,
            0x14 => Self::MethodReturn,
            0x15 => Self::Receiver,
            0x16 => Self::FormalParameter {
                formal_parameter_index: decode_value(bytes)?,
            },
            0x17 => Self::Throws {
                throws_type_index: decode_value(bytes)?,
            },
            0x40 => {
                let table_length: u16 = decode_value(bytes)?;
                let table = borrow_slice(bytes, usize::from(table_length))?;
                Self::LocalVariable(table)
            }
            0x41 => {
                let table_length: u16 = decode_value(bytes)?;
                let table = borrow_slice(bytes, usize::from(table_length))?;
                Self::ResourceVariable(table)
            }
            0x42 => Self::Catch {
                exception_table_index: decode_value(bytes)?,
            },
            0x43 => Self::InstanceOf {
                offset: decode_value(bytes)?,
            },
            0x44 => Self::New {
                offset: decode_value(bytes)?,
            },
            0x45 => Self::ConstructorReference {
                offset: decode_value(bytes)?,
            },
            0x46 => Self::MethodReference {
                offset: decode_value(bytes)?,
            },
            0x47 => Self::TypeInCast {
                offset: decode_value(bytes)?,
                type_argument_index: decode_value(bytes)?,
            },
            0x48 => Self::TypeArgumentInConstructor {
                offset: decode_value(bytes)?,
                type_argument_index: decode_value(bytes)?,
            },
            0x49 => Self::TypeArgumentInCall {
                offset: decode_value(bytes)?,
                type_argument_index: decode_value(bytes)?,
            },
            0x4a => Self::TypeArgumentInConstructorReference {
                offset: decode_value(bytes)?,
                type_argument_index: decode_value(bytes)?,
            },
            0x4b => Self::TypeArgumentInMethodReference {
                offset: decode_value(bytes)?,
                type_argument_index: decode_value(bytes)?,
            },
            unexpected => Err(ParseError::malform(format!(
                "Invalid target type: {unexpected:x}"
            )))?,
        };
        Ok(target_info)
    }
}

#[derive(Debug, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
pub struct LocalVariableTarget {
    pub start_pc: U16,
    pub length: U16,
    pub index: U16,
}

impl FixedLayout for LocalVariableTarget {}
