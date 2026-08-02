//! Raw attribute types for the `Code` attribute and its sub-attributes.

use std::io::{self, Read, Write};
use std::result::Result;

use super::super::{
    FromBytecode, GenerationError, ToBytecode,
    attribute::AttributeInfo,
    reader::{BytecodeReader, read_vec},
    write_length,
};
use crate::{
    intrinsics::{enum_discriminant, see_jvm_spec},
    jvm::code::ProgramCounter,
};

/// The `Code` attribute.
#[doc = see_jvm_spec!(4, 7, 3)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub instruction_bytes: Vec<u8>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub attributes: Vec<AttributeInfo>,
}

impl FromBytecode for Code {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> std::io::Result<Self> {
        let max_stack = reader.decode_value()?;
        let max_locals = reader.decode_value()?;
        let code_length: u32 = reader.decode_value()?;
        let code_length = usize::try_from(code_length).expect("32-bit size is not supported.");
        let instruction_bytes = read_vec(reader, code_length)?;
        let exception_table_length: u16 = reader.decode_value()?;
        let exception_table = (0..exception_table_length)
            .map(|_| reader.decode_value())
            .collect::<io::Result<Vec<_>>>()?;
        let attributes_count: u16 = reader.decode_value()?;
        let attributes = (0..attributes_count)
            .map(|_| reader.decode_value())
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

impl ToBytecode for Code {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.max_stack.to_be_bytes())?;
        writer.write_all(&self.max_locals.to_be_bytes())?;
        write_length::<u32>(writer, self.instruction_bytes.len())?;
        writer.write_all(&self.instruction_bytes)?;
        write_length::<u16>(writer, self.exception_table.len())?;
        for entry in &self.exception_table {
            entry.to_writer(writer)?;
        }
        self.attributes.to_writer(writer)?;
        Ok(())
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

impl FromBytecode for ExceptionTableEntry {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            start_pc: reader.decode_value()?,
            end_pc: reader.decode_value()?,
            handler_pc: reader.decode_value()?,
            catch_type_idx: reader.decode_value()?,
        })
    }
}

impl ToBytecode for ExceptionTableEntry {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.start_pc.to_be_bytes())?;
        writer.write_all(&self.end_pc.to_be_bytes())?;
        writer.write_all(&self.handler_pc.to_be_bytes())?;
        writer.write_all(&self.catch_type_idx.to_be_bytes())?;
        Ok(())
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

impl FromBytecode for StackMapFrameInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let frame_type: u8 = reader.decode_value()?;
        match frame_type {
            frame_type @ 0..=63 => Ok(Self::SameFrame { frame_type }),
            frame_type @ 64..=127 => Ok(Self::SameLocals1StackItemFrame {
                frame_type,
                stack: reader.decode_value()?,
            }),
            frame_type @ 128..=246 => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Frame type {frame_type} is reserved for future use."),
            )),
            247 => Ok(Self::SameLocals1StackItemFrameExtended {
                offset_delta: reader.decode_value()?,
                stack: reader.decode_value()?,
            }),
            frame_type @ 248..=250 => Ok(Self::ChopFrame {
                frame_type,
                offset_delta: reader.decode_value()?,
            }),
            251 => Ok(Self::SameFrameExtended {
                offset_delta: reader.decode_value()?,
            }),
            frame_type @ 252..=254 => {
                let locals_count = frame_type - 251;
                let offset_delta = reader.decode_value()?;
                let locals = (0..locals_count)
                    .map(|_| reader.decode_value())
                    .collect::<io::Result<Vec<_>>>()?;
                Ok(Self::AppendFrame {
                    offset_delta,
                    locals,
                })
            }
            255 => {
                let offset_delta = reader.decode_value()?;
                let number_of_locals: u16 = reader.decode_value()?;
                let locals = (0..number_of_locals)
                    .map(|_| reader.decode_value())
                    .collect::<io::Result<Vec<_>>>()?;
                let number_of_stack_items: u16 = reader.decode_value()?;
                let stack = (0..number_of_stack_items)
                    .map(|_| reader.decode_value())
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

impl ToBytecode for StackMapFrameInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        match self {
            Self::SameFrame { frame_type } => {
                debug_assert!(
                    (0..=63).contains(frame_type),
                    "Invalid frame type in SameFrame"
                );
                writer.write_all(&frame_type.to_be_bytes())?;
            }
            Self::SameLocals1StackItemFrame { frame_type, stack } => {
                debug_assert!(
                    (64..=127).contains(frame_type),
                    "Invalid frame type in SameLocals1StackItemFrame"
                );
                writer.write_all(&frame_type.to_be_bytes())?;
                stack.to_writer(writer)?;
            }
            Self::SameLocals1StackItemFrameExtended {
                offset_delta,
                stack,
            } => {
                writer.write_all(&247u8.to_be_bytes())?;
                writer.write_all(&offset_delta.to_be_bytes())?;
                stack.to_writer(writer)?;
            }
            Self::ChopFrame {
                frame_type,
                offset_delta,
            } => {
                debug_assert!(
                    (248..=250).contains(frame_type),
                    "Invalid frame type in ChopFrame"
                );
                writer.write_all(&frame_type.to_be_bytes())?;
                writer.write_all(&offset_delta.to_be_bytes())?;
            }
            Self::SameFrameExtended { offset_delta } => {
                writer.write_all(&251u8.to_be_bytes())?;
                writer.write_all(&offset_delta.to_be_bytes())?;
            }
            Self::AppendFrame {
                offset_delta,
                locals,
            } => {
                let frame_type = u8::try_from(locals.len() + 251)?;
                debug_assert!(
                    (252..=254).contains(&frame_type),
                    "Invalid frame type in AppendFrame"
                );
                writer.write_all(&frame_type.to_be_bytes())?;
                writer.write_all(&offset_delta.to_be_bytes())?;
                for local in locals {
                    local.to_writer(writer)?;
                }
            }
            Self::FullFrame {
                offset_delta,
                locals,
                stack,
            } => {
                writer.write_all(&255u8.to_be_bytes())?;
                writer.write_all(&offset_delta.to_be_bytes())?;
                write_length::<u16>(writer, locals.len())?;
                for local in locals {
                    local.to_writer(writer)?;
                }
                write_length::<u16>(writer, stack.len())?;
                for value in stack {
                    value.to_writer(writer)?;
                }
            }
        }
        Ok(())
    }
}

#[repr(u8)]
pub enum VerificationTypeInfo {
    Top = 0,
    Integer = 1,
    Float = 2,
    Double = 3,
    Long = 4,
    Null = 5,
    UninitializedThis = 6,
    Object { class_info_index: u16 } = 7,
    Uninitialized { offset: u16 } = 8,
}

impl VerificationTypeInfo {
    const fn tag(&self) -> u8 {
        // SAFETY: Self is repr(u8), so it is fine to call enum_discriminant.
        unsafe { enum_discriminant(self) }
    }
}

impl FromBytecode for VerificationTypeInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let tag: u8 = reader.decode_value()?;
        match tag {
            0 => Ok(Self::Top),
            1 => Ok(Self::Integer),
            2 => Ok(Self::Float),
            3 => Ok(Self::Double),
            4 => Ok(Self::Long),
            5 => Ok(Self::Null),
            6 => Ok(Self::UninitializedThis),
            7 => Ok(Self::Object {
                class_info_index: reader.decode_value()?,
            }),
            8 => Ok(Self::Uninitialized {
                offset: reader.decode_value()?,
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown verification type tag: {tag}"),
            )),
        }
    }
}

impl ToBytecode for VerificationTypeInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        let tag = self.tag();
        writer.write_all(&tag.to_be_bytes())?;
        match self {
            Self::Object {
                class_info_index: value,
            }
            | Self::Uninitialized { offset: value } => writer.write_all(&value.to_be_bytes())?,
            _ => {}
        }
        Ok(())
    }
}

pub struct LocalVariableInfo {
    pub start_pc: ProgramCounter,
    pub length: u16,
    pub name_index: u16,
    pub desc_or_signature_idx: u16,
    pub index: u16,
}

impl FromBytecode for LocalVariableInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            start_pc: reader.decode_value()?,
            length: reader.decode_value()?,
            name_index: reader.decode_value()?,
            desc_or_signature_idx: reader.decode_value()?,
            index: reader.decode_value()?,
        })
    }
}

impl ToBytecode for LocalVariableInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&u16::from(self.start_pc).to_be_bytes())?;
        writer.write_all(&self.length.to_be_bytes())?;
        writer.write_all(&self.name_index.to_be_bytes())?;
        writer.write_all(&self.desc_or_signature_idx.to_be_bytes())?;
        writer.write_all(&self.index.to_be_bytes())?;
        Ok(())
    }
}
