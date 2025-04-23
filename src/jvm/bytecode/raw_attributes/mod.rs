use std::{
    io,
    io::{Write, prelude::Read},
    result::Result,
};

use super::{
    FromReader, ToWriter,
    attribute::AttributeInfo,
    errors::GenerationError,
    reader_utils::{ValueReaderExt, read_byte_chunk},
    write_length,
};
use crate::{jvm::code::ProgramCounter, macros::see_jvm_spec, utils::enum_discriminant};

/// The `Code` attribute.
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
        let code_length = usize::try_from(code_length).expect("32-bit size is not supported.");
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

impl ToWriter for Code {
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

impl ToWriter for ExceptionTableEntry {
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

impl ToWriter for StackMapFrameInfo {
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
        // Safety: Self is repr(u8), so it is fine to call enum_discriminant.
        unsafe { enum_discriminant(self) }
    }
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

impl ToWriter for VerificationTypeInfo {
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

pub struct InnerClass {
    pub info_index: u16,
    pub outer_class_info_index: u16,
    pub inner_name_index: u16,
    pub access_flags: u16,
}

impl FromReader for InnerClass {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            info_index: reader.read_value()?,
            outer_class_info_index: reader.read_value()?,
            inner_name_index: reader.read_value()?,
            access_flags: reader.read_value()?,
        })
    }
}

impl ToWriter for InnerClass {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.info_index.to_be_bytes())?;
        writer.write_all(&self.outer_class_info_index.to_be_bytes())?;
        writer.write_all(&self.inner_name_index.to_be_bytes())?;
        writer.write_all(&self.access_flags.to_be_bytes())?;
        Ok(())
    }
}

pub struct EnclosingMethod {
    pub class_index: u16,
    pub method_index: u16,
}

impl FromReader for EnclosingMethod {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            class_index: reader.read_value()?,
            method_index: reader.read_value()?,
        })
    }
}

impl ToWriter for EnclosingMethod {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.class_index.to_be_bytes())?;
        writer.write_all(&self.method_index.to_be_bytes())?;
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

impl FromReader for LocalVariableInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            start_pc: reader.read_value()?,
            length: reader.read_value()?,
            name_index: reader.read_value()?,
            desc_or_signature_idx: reader.read_value()?,
            index: reader.read_value()?,
        })
    }
}

impl ToWriter for LocalVariableInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&u16::from(self.start_pc).to_be_bytes())?;
        writer.write_all(&self.length.to_be_bytes())?;
        writer.write_all(&self.name_index.to_be_bytes())?;
        writer.write_all(&self.desc_or_signature_idx.to_be_bytes())?;
        writer.write_all(&self.index.to_be_bytes())?;
        Ok(())
    }
}

pub struct Annotation {
    pub type_index: u16,
    pub element_value_pairs: Vec<(u16, ElementValueInfo)>,
}

impl FromReader for Annotation {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let type_index = reader.read_value()?;
        let num_element_value_pairs: u16 = reader.read_value()?;
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_index = reader.read_value()?;
                let element_value = reader.read_value()?;
                Ok((element_name_index, element_value))
            })
            .collect::<io::Result<_>>()?;
        Ok(Self {
            type_index,
            element_value_pairs,
        })
    }
}

impl ToWriter for Annotation {
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

impl FromReader for ElementValueInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let tag: u8 = reader.read_value()?;
        match tag {
            tag @ (b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's') => {
                Ok(Self::Const(tag, reader.read_value()?))
            }
            b'e' => Ok(Self::Enum {
                type_name_index: reader.read_value()?,
                const_name_index: reader.read_value()?,
            }),
            b'c' => Ok(Self::ClassInfo(reader.read_value()?)),
            b'@' => Ok(Self::Annotation(reader.read_value()?)),
            b'[' => {
                let num_values: u16 = reader.read_value()?;
                let values = (0..num_values)
                    .map(|_| reader.read_value())
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

impl ToWriter for ElementValueInfo {
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

impl FromReader for TypeAnnotation {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let target_info = reader.read_value()?;
        let target_path_length: u8 = reader.read_value()?;
        let target_path = (0..target_path_length)
            .map(|_| {
                let type_path_kind = reader.read_value()?;
                let type_argument_index = reader.read_value()?;
                Ok((type_path_kind, type_argument_index))
            })
            .collect::<io::Result<_>>()?;
        let type_index = reader.read_value()?;
        let num_element_value_pairs: u16 = reader.read_value()?;
        let element_value_pairs = (0..num_element_value_pairs)
            .map(|_| {
                let element_name_index = reader.read_value()?;
                let element_value = reader.read_value()?;
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

impl ToWriter for TypeAnnotation {
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

impl FromReader for TargetInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let target_type: u8 = reader.read_value()?;
        let target_info = match target_type {
            0x00 => Self::TypeParameterOfClass {
                index: reader.read_value()?,
            },
            0x01 => Self::TypeParameterOfMethod {
                index: reader.read_value()?,
            },
            0x10 => Self::SuperType {
                index: reader.read_value()?,
            },
            0x11 => Self::TypeParameterBoundOfClass {
                type_parameter_index: reader.read_value()?,
                bound_index: reader.read_value()?,
            },
            0x12 => Self::TypeParameterBoundOfMethod {
                type_parameter_index: reader.read_value()?,
                bound_index: reader.read_value()?,
            },
            0x13 => Self::Field,
            0x14 => Self::TypeOfField,
            0x15 => Self::Receiver,
            0x16 => Self::FormalParameter {
                index: reader.read_value()?,
            },
            0x17 => Self::Throws {
                index: reader.read_value()?,
            },
            0x40 => {
                let table_length: u16 = reader.read_value()?;
                let table = (0..table_length)
                    .map(|_| {
                        let start_pc = reader.read_value()?;
                        let length = reader.read_value()?;
                        let index = reader.read_value()?;
                        Ok((start_pc, length, index))
                    })
                    .collect::<io::Result<_>>()?;
                Self::LocalVariable(table)
            }
            0x41 => {
                let table_length: u16 = reader.read_value()?;
                let table = (0..table_length)
                    .map(|_| {
                        let start_pc = reader.read_value()?;
                        let length = reader.read_value()?;
                        let index = reader.read_value()?;
                        Ok((start_pc, length, index))
                    })
                    .collect::<io::Result<_>>()?;
                Self::ResourceVariable(table)
            }
            0x42 => Self::Catch {
                index: reader.read_value()?,
            },
            0x43 => Self::InstanceOf {
                offset: reader.read_value()?,
            },
            0x44 => Self::New {
                offset: reader.read_value()?,
            },
            0x45 => Self::NewMethodReference {
                offset: reader.read_value()?,
            },
            0x46 => Self::VarMethodReference {
                offset: reader.read_value()?,
            },
            0x47 => Self::TypeInCast {
                offset: reader.read_value()?,
                index: reader.read_value()?,
            },
            0x48 => Self::TypeArgumentInConstructor {
                offset: reader.read_value()?,
                index: reader.read_value()?,
            },
            0x49 => Self::TypeArgumentInCall {
                offset: reader.read_value()?,
                index: reader.read_value()?,
            },
            0x4a => Self::TypeArgumentInConstructorReference {
                offset: reader.read_value()?,
                index: reader.read_value()?,
            },
            0x4b => Self::TypeArgumentInMethodReference {
                offset: reader.read_value()?,
                index: reader.read_value()?,
            },
            unexpected => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid target type: {unexpected:x}"),
            ))?,
        };
        Ok(target_info)
    }
}

impl ToWriter for TargetInfo {
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

pub struct BootstrapMethod {
    pub method_ref_idx: u16,
    pub arguments: Vec<u16>,
}

impl FromReader for BootstrapMethod {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let method_ref_idx = reader.read_value()?;
        let num_arguments: u16 = reader.read_value()?;
        let arguments = (0..num_arguments)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            method_ref_idx,
            arguments,
        })
    }
}

impl ToWriter for BootstrapMethod {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.method_ref_idx.to_be_bytes())?;
        write_length::<u16>(writer, self.arguments.len())?;
        for argument in &self.arguments {
            writer.write_all(&argument.to_be_bytes())?;
        }
        Ok(())
    }
}

pub struct ParameterInfo {
    pub name_index: u16,
    pub access_flags: u16,
}

impl FromReader for ParameterInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            name_index: reader.read_value()?,
            access_flags: reader.read_value()?,
        })
    }
}

impl ToWriter for ParameterInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.name_index.to_be_bytes())?;
        writer.write_all(&self.access_flags.to_be_bytes())?;
        Ok(())
    }
}

pub struct ModuleInfo {
    pub info_index: u16,
    pub flags: u16,
    pub version_index: u16,
    pub requires: Vec<RequiresInfo>,
    pub exports: Vec<ExportsInfo>,
    pub opens: Vec<OpensInfo>,
    pub uses: Vec<u16>,
    pub provides: Vec<ProvidesInfo>,
}

impl FromReader for ModuleInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let info_index = reader.read_value()?;
        let flags = reader.read_value()?;
        let version_index = reader.read_value()?;
        let requires_count: u16 = reader.read_value()?;
        let requires = (0..requires_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        let exports_count: u16 = reader.read_value()?;
        let exports = (0..exports_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        let opens_count: u16 = reader.read_value()?;
        let opens = (0..opens_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        let uses_count: u16 = reader.read_value()?;
        let uses = (0..uses_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        let provides_count: u16 = reader.read_value()?;
        let provides = (0..provides_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            info_index,
            flags,
            version_index,
            requires,
            exports,
            opens,
            uses,
            provides,
        })
    }
}

impl ToWriter for ModuleInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.info_index.to_be_bytes())?;
        writer.write_all(&self.flags.to_be_bytes())?;
        writer.write_all(&self.version_index.to_be_bytes())?;
        write_length::<u16>(writer, self.requires.len())?;
        for require in &self.requires {
            require.to_writer(writer)?;
        }
        write_length::<u16>(writer, self.exports.len())?;
        for export in &self.exports {
            export.to_writer(writer)?;
        }
        write_length::<u16>(writer, self.opens.len())?;
        for open in &self.opens {
            open.to_writer(writer)?;
        }
        write_length::<u16>(writer, self.uses.len())?;
        for use_ in &self.uses {
            writer.write_all(&use_.to_be_bytes())?;
        }
        write_length::<u16>(writer, self.provides.len())?;
        for provide in &self.provides {
            provide.to_writer(writer)?;
        }
        Ok(())
    }
}

pub struct RequiresInfo {
    pub requires_index: u16,
    pub flags: u16,
    pub version_index: u16,
}

impl FromReader for RequiresInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            requires_index: reader.read_value()?,
            flags: reader.read_value()?,
            version_index: reader.read_value()?,
        })
    }
}

impl ToWriter for RequiresInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.requires_index.to_be_bytes())?;
        writer.write_all(&self.flags.to_be_bytes())?;
        writer.write_all(&self.version_index.to_be_bytes())?;
        Ok(())
    }
}

pub struct ExportsInfo {
    pub exports_index: u16,
    pub flags: u16,
    pub to: Vec<u16>,
}

impl FromReader for ExportsInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let exports_index = reader.read_value()?;
        let flags = reader.read_value()?;
        let to_count: u16 = reader.read_value()?;
        let to = (0..to_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            exports_index,
            flags,
            to,
        })
    }
}

impl ToWriter for ExportsInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.exports_index.to_be_bytes())?;
        writer.write_all(&self.flags.to_be_bytes())?;
        write_length::<u16>(writer, self.to.len())?;
        for to in &self.to {
            writer.write_all(&to.to_be_bytes())?;
        }
        Ok(())
    }
}

pub struct OpensInfo {
    pub opens_index: u16,
    pub flags: u16,
    pub to: Vec<u16>,
}

impl FromReader for OpensInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let opens_index = reader.read_value()?;
        let flags = reader.read_value()?;
        let to_count: u16 = reader.read_value()?;
        let to = (0..to_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            opens_index,
            flags,
            to,
        })
    }
}

impl ToWriter for OpensInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.opens_index.to_be_bytes())?;
        writer.write_all(&self.flags.to_be_bytes())?;
        write_length::<u16>(writer, self.to.len())?;
        for to in &self.to {
            writer.write_all(&to.to_be_bytes())?;
        }
        Ok(())
    }
}

pub struct ProvidesInfo {
    pub provides_index: u16,
    pub with: Vec<u16>,
}

impl FromReader for ProvidesInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let provides_index = reader.read_value()?;
        let with_count: u16 = reader.read_value()?;
        let with = (0..with_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            provides_index,
            with,
        })
    }
}

impl ToWriter for ProvidesInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.provides_index.to_be_bytes())?;
        write_length::<u16>(writer, self.with.len())?;
        for with in &self.with {
            writer.write_all(&with.to_be_bytes())?;
        }
        Ok(())
    }
}

pub struct RecordComponentInfo {
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeInfo>,
}

impl FromReader for RecordComponentInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let name_index = reader.read_value()?;
        let descriptor_index = reader.read_value()?;
        let attributes_count: u16 = reader.read_value()?;
        let attributes = (0..attributes_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl ToWriter for RecordComponentInfo {
    fn to_writer<W>(&self, writer: &mut W) -> Result<(), GenerationError>
    where
        W: Write + ?Sized,
    {
        writer.write_all(&self.name_index.to_be_bytes())?;
        writer.write_all(&self.descriptor_index.to_be_bytes())?;
        self.attributes.to_writer(writer)?;
        Ok(())
    }
}
