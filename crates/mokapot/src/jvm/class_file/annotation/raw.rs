//! Raw type-annotation targets and paths.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::{
    Decode, ParseError,
    decoder::{borrow_slice, decode_value},
    fixed_layout::FixedLayout,
};
use crate::intrinsics::see_jvm_spec;

#[derive(Debug, Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct TypePathElement {
    pub kind: u8,
    pub argument_index: u8,
}

impl FixedLayout for TypePathElement {}

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
