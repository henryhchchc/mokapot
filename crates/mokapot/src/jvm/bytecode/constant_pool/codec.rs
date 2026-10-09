use std::io::{self, Read, Write};

use super::RawEntry;
use crate::jvm::{
    bytecode::{
        ToBytecode,
        reader::{BytecodeReader, read_vec},
        write_length,
    },
    errors::GenerationError,
};

impl RawEntry {
    pub(super) fn parse<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let tag: u8 = reader.decode_value()?;
        match tag {
            1 => Self::parse_utf8(reader),
            3 => reader.decode_value().map(Self::Integer),
            4 => reader.decode_value().map(Self::Float),
            5 => reader.decode_value().map(Self::Long),
            6 => reader.decode_value().map(Self::Double),
            7 => reader
                .decode_value()
                .map(|name_index| Self::Class { name_index }),
            8 => reader
                .decode_value()
                .map(|string_index| Self::String { string_index }),
            9 => Ok(Self::FieldRef {
                class_index: reader.decode_value()?,
                name_and_type_index: reader.decode_value()?,
            }),
            10 => Ok(Self::MethodRef {
                class_index: reader.decode_value()?,
                name_and_type_index: reader.decode_value()?,
            }),
            11 => Ok(Self::InterfaceMethodRef {
                class_index: reader.decode_value()?,
                name_and_type_index: reader.decode_value()?,
            }),
            12 => Ok(Self::NameAndType {
                name_index: reader.decode_value()?,
                descriptor_index: reader.decode_value()?,
            }),
            15 => Ok(Self::MethodHandle {
                reference_kind: reader.decode_value()?,
                reference_index: reader.decode_value()?,
            }),
            16 => Ok(Self::MethodType {
                descriptor_index: reader.decode_value()?,
            }),
            17 => Ok(Self::Dynamic {
                bootstrap_method_attr_index: reader.decode_value()?,
                name_and_type_index: reader.decode_value()?,
            }),
            18 => Ok(Self::InvokeDynamic {
                bootstrap_method_attr_index: reader.decode_value()?,
                name_and_type_index: reader.decode_value()?,
            }),
            19 => reader
                .decode_value()
                .map(|name_index| Self::Module { name_index }),
            20 => reader
                .decode_value()
                .map(|name_index| Self::Package { name_index }),
            tag => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid constant pool tag: {tag}"),
            )),
        }
    }

    fn parse_utf8<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let length: u16 = reader.decode_value()?;
        let bytes = read_vec(reader, length.into())?;
        Ok(Self::Utf8(bytes.into_boxed_slice()))
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
