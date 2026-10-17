//! Raw attribute types for the `Code` attribute and its sub-attributes.

use zerocopy::byteorder::big_endian::U32;

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    FromBytecode, ParseError,
    attribute::AttributeInfo,
    decoder::{BytecodeSlice, read_bytes, read_value},
};
use crate::intrinsics::see_jvm_spec;

/// The `Code` attribute.
#[doc = see_jvm_spec!(4, 7, 3)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Box<[u8]>,
    pub exception_table: Box<[ExceptionTableEntry]>,
    pub attributes: Box<[AttributeInfo]>,
}

impl FromBytecode for Code {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let max_stack = read_value::<U16>(bytes)?.get();
        let max_locals = read_value::<U16>(bytes)?.get();
        let code_length: u32 = read_value::<U32>(bytes)?.get();
        let code_length = usize::try_from(code_length).expect("32-bit size is not supported.");
        let code = read_bytes(bytes, code_length)?;
        let exception_table_length: u16 = read_value::<U16>(bytes)?.get();
        let exception_table = (0..exception_table_length)
            .map(|_| bytes.decode_value())
            .collect::<Result<_, ParseError>>()?;
        let attributes_count: u16 = read_value::<U16>(bytes)?.get();
        let attributes = (0..attributes_count)
            .map(|_| bytes.decode_value())
            .collect::<Result<_, ParseError>>()?;
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
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
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
        locals: Vec<VerificationTypeInfo>,
    },
    FullFrame {
        // frame_type: u8 = 255,
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
        stack: Vec<VerificationTypeInfo>,
    },
}

impl FromBytecode for StackMapFrameInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let frame_type: u8 = read_value::<u8>(bytes)?;
        match frame_type {
            frame_type @ 0..=63 => Ok(Self::SameFrame { frame_type }),
            frame_type @ 64..=127 => Ok(Self::SameLocals1StackItemFrame {
                frame_type,
                stack: bytes.decode_value()?,
            }),
            frame_type @ 128..=246 => Err(ParseError::malform(format!(
                "Frame type {frame_type} is reserved for future use."
            ))),
            247 => Ok(Self::SameLocals1StackItemFrameExtended {
                offset_delta: read_value::<U16>(bytes)?.get(),
                stack: bytes.decode_value()?,
            }),
            frame_type @ 248..=250 => Ok(Self::ChopFrame {
                frame_type,
                offset_delta: read_value::<U16>(bytes)?.get(),
            }),
            251 => Ok(Self::SameFrameExtended {
                offset_delta: read_value::<U16>(bytes)?.get(),
            }),
            frame_type @ 252..=254 => {
                let locals_count = frame_type - 251;
                let offset_delta = read_value::<U16>(bytes)?.get();
                let locals = (0..locals_count)
                    .map(|_| bytes.decode_value())
                    .collect::<Result<Vec<_>, ParseError>>()?;
                Ok(Self::AppendFrame {
                    offset_delta,
                    locals,
                })
            }
            255 => {
                let offset_delta = read_value::<U16>(bytes)?.get();
                let number_of_locals: u16 = read_value::<U16>(bytes)?.get();
                let locals = (0..number_of_locals)
                    .map(|_| bytes.decode_value())
                    .collect::<Result<Vec<_>, ParseError>>()?;
                let number_of_stack_items: u16 = read_value::<U16>(bytes)?.get();
                let stack = (0..number_of_stack_items)
                    .map(|_| bytes.decode_value())
                    .collect::<Result<Vec<_>, ParseError>>()?;
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

impl FromBytecode for VerificationTypeInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let tag: u8 = read_value::<u8>(bytes)?;
        match tag {
            0 => Ok(Self::Top),
            1 => Ok(Self::Integer),
            2 => Ok(Self::Float),
            3 => Ok(Self::Double),
            4 => Ok(Self::Long),
            5 => Ok(Self::Null),
            6 => Ok(Self::UninitializedThis),
            7 => Ok(Self::Object {
                cpool_index: read_value::<U16>(bytes)?.get(),
            }),
            8 => Ok(Self::Uninitialized {
                offset: read_value::<U16>(bytes)?.get(),
            }),
            _ => Err(ParseError::malform(format!(
                "Unknown verification type tag: {tag}"
            ))),
        }
    }
}

/// A line number table entry.
#[doc = see_jvm_spec!(4, 7, 12)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LineNumberTableEntry {
    pub start_pc: U16,
    pub line_number: U16,
}

impl FixedLayout for LineNumberTableEntry {}

#[doc = see_jvm_spec!(4, 7, 13)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
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
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LocalVariableTypeInfo {
    pub start_pc: U16,
    pub length: U16,
    pub name_index: U16,
    pub signature_index: U16,
    pub index: U16,
}

impl FixedLayout for LocalVariableTypeInfo {}
