use zerocopy::{
    FromBytes,
    byteorder::big_endian::{F32, F64, I32, I64, U16},
};

use std::io::{self, Read, Write};

use super::RawEntry;
use crate::jvm::{
    bytecode::{ToBytecode, reader::read_bytes, write_length},
    errors::GenerationError,
};

impl RawEntry {
    pub(super) fn parse<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let tag: u8 = u8::read_from_io(&mut *reader)?;
        match tag {
            1 => Self::parse_utf8(reader),
            3 => I32::read_from_io(&mut *reader).map(Self::Integer),
            4 => F32::read_from_io(&mut *reader).map(Self::Float),
            5 => I64::read_from_io(&mut *reader).map(Self::Long),
            6 => F64::read_from_io(&mut *reader).map(Self::Double),
            7 => U16::read_from_io(&mut *reader).map(|name_index| Self::Class { name_index }),
            8 => U16::read_from_io(&mut *reader).map(|string_index| Self::String { string_index }),
            9 => Ok(Self::FieldRef {
                class_index: U16::read_from_io(&mut *reader)?,
                name_and_type_index: U16::read_from_io(&mut *reader)?,
            }),
            10 => Ok(Self::MethodRef {
                class_index: U16::read_from_io(&mut *reader)?,
                name_and_type_index: U16::read_from_io(&mut *reader)?,
            }),
            11 => Ok(Self::InterfaceMethodRef {
                class_index: U16::read_from_io(&mut *reader)?,
                name_and_type_index: U16::read_from_io(&mut *reader)?,
            }),
            12 => Ok(Self::NameAndType {
                name_index: U16::read_from_io(&mut *reader)?,
                descriptor_index: U16::read_from_io(&mut *reader)?,
            }),
            15 => Ok(Self::MethodHandle {
                reference_kind: u8::read_from_io(&mut *reader)?,
                reference_index: U16::read_from_io(&mut *reader)?,
            }),
            16 => Ok(Self::MethodType {
                descriptor_index: U16::read_from_io(&mut *reader)?,
            }),
            17 => Ok(Self::Dynamic {
                bootstrap_method_attr_index: U16::read_from_io(&mut *reader)?,
                name_and_type_index: U16::read_from_io(&mut *reader)?,
            }),
            18 => Ok(Self::InvokeDynamic {
                bootstrap_method_attr_index: U16::read_from_io(&mut *reader)?,
                name_and_type_index: U16::read_from_io(&mut *reader)?,
            }),
            19 => U16::read_from_io(&mut *reader).map(|name_index| Self::Module { name_index }),
            20 => U16::read_from_io(&mut *reader).map(|name_index| Self::Package { name_index }),
            tag => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid constant pool tag: {tag}"),
            )),
        }
    }

    fn parse_utf8<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let length: u16 = U16::read_from_io(&mut *reader)?.get();
        let bytes = read_bytes(reader, length.into())?;
        Ok(Self::Utf8(bytes))
    }
}

impl ToBytecode for RawEntry {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&[self.tag()])?;
        match self {
            Self::Utf8(value) => {
                write_length::<u16>(writer, value.len())?;
                writer.write_all(value)?;
            }
            Self::Integer(value) => value.to_writer(writer)?,
            Self::Float(value) => value.to_writer(writer)?,
            Self::Long(value) => value.to_writer(writer)?,
            Self::Double(value) => value.to_writer(writer)?,
            Self::Class { name_index }
            | Self::Module { name_index }
            | Self::Package { name_index } => name_index.to_writer(writer)?,
            Self::String { string_index } => string_index.to_writer(writer)?,
            Self::FieldRef {
                class_index,
                name_and_type_index,
            }
            | Self::MethodRef {
                class_index,
                name_and_type_index,
            }
            | Self::InterfaceMethodRef {
                class_index,
                name_and_type_index,
            } => {
                class_index.to_writer(writer)?;
                name_and_type_index.to_writer(writer)?;
            }
            Self::NameAndType {
                name_index,
                descriptor_index,
            } => {
                name_index.to_writer(writer)?;
                descriptor_index.to_writer(writer)?;
            }
            Self::MethodHandle {
                reference_kind,
                reference_index,
            } => {
                writer.write_all(&[*reference_kind])?;
                reference_index.to_writer(writer)?;
            }
            Self::MethodType { descriptor_index } => {
                descriptor_index.to_writer(writer)?;
            }
            Self::Dynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            }
            | Self::InvokeDynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => {
                bootstrap_method_attr_index.to_writer(writer)?;
                name_and_type_index.to_writer(writer)?;
            }
        }
        Ok(())
    }
}
