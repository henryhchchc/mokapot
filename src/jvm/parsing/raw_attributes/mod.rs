use std::io;
use std::io::prelude::Read;

use crate::macros::see_jvm_spec;

use super::attribute::AttributeInfo;
use super::reader_utils::read_byte_chunk;
use super::reader_utils::FromReader;
use super::reader_utils::ValueReaderExt;

/// The `Code` atribute.
#[doc = see_jvm_spec!(4, 7, 3)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub instruction_bytes: Vec<u8>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub attributes: Vec<AttributeInfo>,
}

impl FromReader for Code {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> std::io::Result<Self> {
        let max_stack = reader.read_value()?;
        let max_locals = reader.read_value()?;
        let code_length: u32 = reader.read_value()?;
        let code_length = usize::try_from(code_length).expect("32-bit size is not supportted.");
        let instruction_bytes = read_byte_chunk(reader, code_length)?;
        let exception_table_length: u16 = reader.read_value()?;
        let exception_table = (0..exception_table_length)
            .map(|_| reader.read_value())
            .collect::<io::Result<Vec<_>>>()?;
        let attributes_count: u16 = reader.read_value()?;
        let attributes = (0..attributes_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<Vec<_>>>()?;
        Ok(Self {
            max_stack,
            max_locals,
            instruction_bytes,
            exception_table,
            attributes,
        })
    }
}

/// An entry in the exception table of a `Code` attribute.
#[doc = see_jvm_spec!(4, 7, 3)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type_idx: u16,
}

impl FromReader for ExceptionTableEntry {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            start_pc: reader.read_value()?,
            end_pc: reader.read_value()?,
            handler_pc: reader.read_value()?,
            catch_type_idx: reader.read_value()?,
        })
    }
}

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

impl FromReader for StackMapFrameInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let frame_type: u8 = reader.read_value()?;
        match frame_type {
            frame_type @ 0..=63 => Ok(Self::SameFrame { frame_type }),
            frame_type @ 64..=127 => Ok(Self::SameLocals1StackItemFrame {
                frame_type,
                stack: reader.read_value()?,
            }),
            frame_type @ 128..=246 => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Frame type {frame_type} is reserved for future use."),
            )),
            247 => Ok(Self::SameLocals1StackItemFrameExtended {
                offset_delta: reader.read_value()?,
                stack: reader.read_value()?,
            }),
            frame_type @ 248..=250 => Ok(Self::ChopFrame {
                frame_type,
                offset_delta: reader.read_value()?,
            }),
            251 => Ok(Self::SameFrameExtended {
                offset_delta: reader.read_value()?,
            }),
            frame_type @ 252..=254 => {
                let locals_count = frame_type - 251;
                let offset_delta = reader.read_value()?;
                let locals = (0..locals_count)
                    .map(|_| reader.read_value())
                    .collect::<io::Result<Vec<_>>>()?;
                Ok(Self::AppendFrame {
                    offset_delta,
                    locals,
                })
            }
            255 => {
                let offset_delta = reader.read_value()?;
                let number_of_locals: u16 = reader.read_value()?;
                let locals = (0..number_of_locals)
                    .map(|_| reader.read_value())
                    .collect::<io::Result<Vec<_>>>()?;
                let number_of_stack_items: u16 = reader.read_value()?;
                let stack = (0..number_of_stack_items)
                    .map(|_| reader.read_value())
                    .collect::<io::Result<Vec<_>>>()?;
                Ok(Self::FullFrame {
                    offset_delta,
                    locals,
                    stack,
                })
            }
        }
    }
}

pub enum VerificationTypeInfo {
    Top,
    Integer,
    Float,
    Long,
    Double,
    Null,
    UninitializedThis,
    Object { class_info_index: u16 },
    Uninitialized { offset: u16 },
}

impl FromReader for VerificationTypeInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let tag: u8 = reader.read_value()?;
        match tag {
            0 => Ok(Self::Top),
            1 => Ok(Self::Integer),
            2 => Ok(Self::Float),
            3 => Ok(Self::Double),
            4 => Ok(Self::Long),
            5 => Ok(Self::Null),
            6 => Ok(Self::UninitializedThis),
            7 => Ok(Self::Object {
                class_info_index: reader.read_value()?,
            }),
            8 => Ok(Self::Uninitialized {
                offset: reader.read_value()?,
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown verification type tag: {tag}"),
            )),
        }
    }
}
