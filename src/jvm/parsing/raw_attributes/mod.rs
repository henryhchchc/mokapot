use std::io;
use std::io::prelude::Read;

use crate::jvm::code::ProgramCounter;
use crate::macros::see_jvm_spec;

use super::attribute::AttributeInfo;
use super::reader_utils::FromReader;
use super::reader_utils::ValueReaderExt;
use super::reader_utils::read_byte_chunk;

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

pub enum TargetInfo {
    TypeParameter { index: u8 },
    SuperType { index: u16 },
    TypeParameterBound { type_parameter: u8, bound_index: u8 },
    Empty,
    FormalParameter { index: u8 },
    Throws { index: u16 },
    LocalVariable(Vec<(ProgramCounter, u16, u16)>),
    Catch { exception_table_index: u16 },
    Offset(u16),
    TypeArgument { offset: ProgramCounter, index: u8 },
}

impl FromReader for TargetInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let target_type: u8 = reader.read_value()?;
        let target_info = match target_type {
            0x00 | 0x01 => Self::TypeParameter {
                index: reader.read_value()?,
            },
            0x10 => Self::SuperType {
                index: reader.read_value()?,
            },
            0x11 | 0x12 => Self::TypeParameterBound {
                type_parameter: reader.read_value()?,
                bound_index: reader.read_value()?,
            },
            0x13..=0x15 => Self::Empty,
            0x16 => Self::FormalParameter {
                index: reader.read_value()?,
            },
            0x17 => Self::Throws {
                index: reader.read_value()?,
            },
            0x40 | 0x41 => {
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
            0x42 => Self::Catch {
                exception_table_index: reader.read_value()?,
            },
            0x43..=0x46 => Self::Offset(reader.read_value()?),
            0x47..=0x4B => Self::TypeArgument {
                offset: reader.read_value()?,
                index: reader.read_value()?,
            },
            unexpected => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid target type: {unexpected}"),
            ))?,
        };
        Ok(target_info)
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

pub struct ParameterInfo(pub u16, pub u16);

impl FromReader for ParameterInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self(reader.read_value()?, reader.read_value()?))
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
