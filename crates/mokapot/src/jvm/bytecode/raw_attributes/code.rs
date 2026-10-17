//! Raw attribute types for the `Code` attribute and its sub-attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    FromBytecode, ParseError,
    attribute::AttributeInfo,
    decoder::{borrow_bytes, borrow_slice, parse_repeated, read_u8, read_u16, read_u32},
};
use crate::intrinsics::see_jvm_spec;

/// The `Code` attribute.
#[doc = see_jvm_spec!(4, 7, 3)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
pub struct Code<'a> {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: &'a [u8],
    pub exception_table: &'a [ExceptionTableEntry],
    pub attributes: Box<[AttributeInfo<'a>]>,
}

impl<'a> FromBytecode<'a> for Code<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let max_stack = read_u16(bytes)?;
        let max_locals = read_u16(bytes)?;
        let code_length: u32 = read_u32(bytes)?;
        let code_length = usize::try_from(code_length).expect("32-bit size is not supported.");
        let code = borrow_bytes(bytes, code_length)?;
        let exception_table_length: u16 = read_u16(bytes)?;
        let exception_table = borrow_slice(bytes, usize::from(exception_table_length))?;
        let attributes_count: u16 = read_u16(bytes)?;
        let attributes = parse_repeated(
            bytes,
            usize::from(attributes_count),
            AttributeInfo::from_bytes,
        )?;
        Ok(Self {
            max_stack,
            max_locals,
            code,
            exception_table,
            attributes,
        })
    }
}

/// An entry in the exception table of a `Code` attribute.
#[doc = see_jvm_spec!(4, 7, 3)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ExceptionTableEntry {
    pub start_pc: U16,
    pub end_pc: U16,
    pub handler_pc: U16,
    pub catch_type: U16,
}

impl FixedLayout for ExceptionTableEntry {}

#[doc = see_jvm_spec!(4, 7, 4)]
pub enum StackMapFrameInfo {
    SameFrame {
        frame_type: u8,
    },
    SameLocals1StackItemFrame {
        frame_type: u8,
        stack: VerificationTypeInfo,
    },
    SameLocals1StackItemFrameExtended {
        // frame_type: u8 = 247,
        offset_delta: u16,
        stack: VerificationTypeInfo,
    },
    ChopFrame {
        frame_type: u8,
        offset_delta: u16,
    },
    SameFrameExtended {
        // frame_type: u8 = 251,
        offset_delta: u16,
    },
    AppendFrame {
        offset_delta: u16,
        locals: Box<[VerificationTypeInfo]>,
    },
    FullFrame {
        // frame_type: u8 = 255,
        offset_delta: u16,
        locals: Box<[VerificationTypeInfo]>,
        stack: Box<[VerificationTypeInfo]>,
    },
}

impl<'a> FromBytecode<'a> for StackMapFrameInfo {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let frame_type: u8 = read_u8(bytes)?;
        match frame_type {
            frame_type @ 0..=63 => Ok(Self::SameFrame { frame_type }),
            frame_type @ 64..=127 => Ok(Self::SameLocals1StackItemFrame {
                frame_type,
                stack: VerificationTypeInfo::from_bytes(bytes)?,
            }),
            frame_type @ 128..=246 => Err(ParseError::malform(format!(
                "Frame type {frame_type} is reserved for future use."
            ))),
            247 => Ok(Self::SameLocals1StackItemFrameExtended {
                offset_delta: read_u16(bytes)?,
                stack: VerificationTypeInfo::from_bytes(bytes)?,
            }),
            frame_type @ 248..=250 => Ok(Self::ChopFrame {
                frame_type,
                offset_delta: read_u16(bytes)?,
            }),
            251 => Ok(Self::SameFrameExtended {
                offset_delta: read_u16(bytes)?,
            }),
            frame_type @ 252..=254 => {
                let locals_count = frame_type - 251;
                let offset_delta = read_u16(bytes)?;
                let locals = parse_repeated(
                    bytes,
                    usize::from(locals_count),
                    VerificationTypeInfo::from_bytes,
                )?;
                Ok(Self::AppendFrame {
                    offset_delta,
                    locals,
                })
            }
            255 => {
                let offset_delta = read_u16(bytes)?;
                let number_of_locals: u16 = read_u16(bytes)?;
                let locals = parse_repeated(
                    bytes,
                    usize::from(number_of_locals),
                    VerificationTypeInfo::from_bytes,
                )?;
                let number_of_stack_items: u16 = read_u16(bytes)?;
                let stack = parse_repeated(
                    bytes,
                    usize::from(number_of_stack_items),
                    VerificationTypeInfo::from_bytes,
                )?;
                Ok(Self::FullFrame {
                    offset_delta,
                    locals,
                    stack,
                })
            }
        }
    }
}

#[doc = see_jvm_spec!(4, 7, 4)]
#[repr(u8)]
pub enum VerificationTypeInfo {
    Top = 0,
    Integer = 1,
    Float = 2,
    Double = 3,
    Long = 4,
    Null = 5,
    UninitializedThis = 6,
    Object { cpool_index: u16 } = 7,
    Uninitialized { offset: u16 } = 8,
}

impl<'a> FromBytecode<'a> for VerificationTypeInfo {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let tag: u8 = read_u8(bytes)?;
        match tag {
            0 => Ok(Self::Top),
            1 => Ok(Self::Integer),
            2 => Ok(Self::Float),
            3 => Ok(Self::Double),
            4 => Ok(Self::Long),
            5 => Ok(Self::Null),
            6 => Ok(Self::UninitializedThis),
            7 => Ok(Self::Object {
                cpool_index: read_u16(bytes)?,
            }),
            8 => Ok(Self::Uninitialized {
                offset: read_u16(bytes)?,
            }),
            _ => Err(ParseError::malform(format!(
                "Unknown verification type tag: {tag}"
            ))),
        }
    }
}

/// A line number table entry.
#[doc = see_jvm_spec!(4, 7, 12)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LineNumberTableEntry {
    pub start_pc: U16,
    pub line_number: U16,
}

impl FixedLayout for LineNumberTableEntry {}

#[doc = see_jvm_spec!(4, 7, 13)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LocalVariableInfo {
    pub start_pc: U16,
    pub length: U16,
    pub name_index: U16,
    pub descriptor_index: U16,
    pub index: U16,
}

impl FixedLayout for LocalVariableInfo {}

#[doc = see_jvm_spec!(4, 7, 14)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LocalVariableTypeInfo {
    pub start_pc: U16,
    pub length: U16,
    pub name_index: U16,
    pub signature_index: U16,
    pub index: U16,
}

impl FixedLayout for LocalVariableTypeInfo {}
