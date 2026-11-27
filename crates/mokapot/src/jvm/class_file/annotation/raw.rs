//! Raw attribute types for annotation-related attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::{
    Decode, ParseError,
    decoder::{parse_repeated, try_borrow_view},
};
use crate::{intrinsics::see_jvm_spec, jvm::ConstantPoolIndex};

#[doc = see_jvm_spec!(4, 7, 16)]
pub struct Annotation {
    pub annotation_type: ConstantPoolIndex,
    pub element_value_pairs: Box<[(ConstantPoolIndex, ElementValueInfo)]>,
}

impl<'a> Decode<'a> for Annotation {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let annotation_type = Decode::decode(bytes)?;
        let num_element_value_pairs = u16::decode(bytes)?;
        let element_value_pairs = parse_repeated(bytes, num_element_value_pairs, |bytes| {
            let element_name = Decode::decode(bytes)?;
            let element_value = ElementValueInfo::decode(bytes)?;
            Ok((element_name, element_value))
        })?;
        Ok(Self {
            annotation_type,
            element_value_pairs,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 16, 1)]
pub enum ElementValueInfo {
    Const(u8, ConstantPoolIndex),
    Enum {
        type_name: ConstantPoolIndex,
        const_name: ConstantPoolIndex,
    },
    ClassInfo(ConstantPoolIndex),
    Annotation(Annotation),
    Array(Box<[ElementValueInfo]>),
}

impl<'a> Decode<'a> for ElementValueInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let tag = u8::decode(bytes)?;
        match tag {
            tag @ (b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's') => {
                Ok(Self::Const(tag, Decode::decode(bytes)?))
            }
            b'e' => Ok(Self::Enum {
                type_name: Decode::decode(bytes)?,
                const_name: Decode::decode(bytes)?,
            }),
            b'c' => Ok(Self::ClassInfo(Decode::decode(bytes)?)),
            b'@' => Ok(Self::Annotation(Annotation::decode(bytes)?)),
            b'[' => {
                let num_values = u16::decode(bytes)?;
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

#[doc = see_jvm_spec!(4, 7, 20)]
pub struct TypeAnnotation<'a> {
    pub target_info: TargetInfo<'a>,
    pub target_path: &'a [TypePathElement],
    pub annotation_type: ConstantPoolIndex,
    pub element_value_pairs: Box<[(ConstantPoolIndex, ElementValueInfo)]>,
}

impl<'a> Decode<'a> for TypeAnnotation<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let target_info = TargetInfo::decode(bytes)?;
        let target_path_length = u8::decode(bytes)?;
        let target_path = try_borrow_view(bytes, usize::from(target_path_length))?;
        let annotation_type = Decode::decode(bytes)?;
        let num_element_value_pairs = u16::decode(bytes)?;
        let element_value_pairs = parse_repeated(bytes, num_element_value_pairs, |bytes| {
            let element_name = Decode::decode(bytes)?;
            let element_value = ElementValueInfo::decode(bytes)?;
            Ok((element_name, element_value))
        })?;
        Ok(Self {
            target_info,
            target_path,
            annotation_type,
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
        let target_type = u8::decode(bytes)?;
        let target_info = match target_type {
            0x00 => Self::TypeParameterOfClass {
                type_parameter_index: Decode::decode(bytes)?,
            },
            0x01 => Self::TypeParameterOfMethod {
                type_parameter_index: Decode::decode(bytes)?,
            },
            0x10 => Self::SuperType {
                supertype_index: Decode::decode(bytes)?,
            },
            0x11 => Self::TypeParameterBoundOfClass {
                type_parameter_index: Decode::decode(bytes)?,
                bound_index: Decode::decode(bytes)?,
            },
            0x12 => Self::TypeParameterBoundOfMethod {
                type_parameter_index: Decode::decode(bytes)?,
                bound_index: Decode::decode(bytes)?,
            },
            0x13 => Self::Field,
            0x14 => Self::MethodReturn,
            0x15 => Self::Receiver,
            0x16 => Self::FormalParameter {
                formal_parameter_index: Decode::decode(bytes)?,
            },
            0x17 => Self::Throws {
                throws_type_index: Decode::decode(bytes)?,
            },
            0x40 => {
                let table_length = u16::decode(bytes)?;
                let table = try_borrow_view(bytes, usize::from(table_length))?;
                Self::LocalVariable(table)
            }
            0x41 => {
                let table_length = u16::decode(bytes)?;
                let table = try_borrow_view(bytes, usize::from(table_length))?;
                Self::ResourceVariable(table)
            }
            0x42 => Self::Catch {
                exception_table_index: Decode::decode(bytes)?,
            },
            0x43 => Self::InstanceOf {
                offset: Decode::decode(bytes)?,
            },
            0x44 => Self::New {
                offset: Decode::decode(bytes)?,
            },
            0x45 => Self::ConstructorReference {
                offset: Decode::decode(bytes)?,
            },
            0x46 => Self::MethodReference {
                offset: Decode::decode(bytes)?,
            },
            0x47 => Self::TypeInCast {
                offset: Decode::decode(bytes)?,
                type_argument_index: Decode::decode(bytes)?,
            },
            0x48 => Self::TypeArgumentInConstructor {
                offset: Decode::decode(bytes)?,
                type_argument_index: Decode::decode(bytes)?,
            },
            0x49 => Self::TypeArgumentInCall {
                offset: Decode::decode(bytes)?,
                type_argument_index: Decode::decode(bytes)?,
            },
            0x4a => Self::TypeArgumentInConstructorReference {
                offset: Decode::decode(bytes)?,
                type_argument_index: Decode::decode(bytes)?,
            },
            0x4b => Self::TypeArgumentInMethodReference {
                offset: Decode::decode(bytes)?,
                type_argument_index: Decode::decode(bytes)?,
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
