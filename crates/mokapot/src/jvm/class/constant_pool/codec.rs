use std::io::{self, Read, Write};

use super::Entry;
use crate::jvm::{
    JavaString,
    bytecode::{
        ToBytecode,
        reader::{BytecodeReader, read_vec},
        write_length,
    },
    errors::GenerationError,
};

impl Entry {
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
        Ok(match cesu8::from_java_cesu8(&bytes) {
            Ok(value) => Self::Utf8(JavaString::Utf8(value.into_owned())),
            Err(_) => Self::Utf8(JavaString::InvalidUtf8(bytes)),
        })
    }
}

impl ToBytecode for JavaString {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        match self {
            Self::Utf8(value) => {
                let bytes = cesu8::to_java_cesu8(value);
                write_length::<u16>(writer, bytes.len())?;
                writer.write_all(bytes.as_ref())?;
            }
            Self::InvalidUtf8(bytes) => {
                write_length::<u16>(writer, bytes.len())?;
                writer.write_all(bytes)?;
            }
        }
        Ok(())
    }
}

impl ToBytecode for Entry {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&[self.tag()])?;
        match self {
            Self::Utf8(value) => value.to_writer(writer)?,
            Self::Integer(value) => writer.write_all(&value.to_be_bytes())?,
            Self::Float(value) => writer.write_all(&value.to_be_bytes())?,
            Self::Long(value) => writer.write_all(&value.to_be_bytes())?,
            Self::Double(value) => writer.write_all(&value.to_be_bytes())?,
            Self::Class { name_index }
            | Self::Module { name_index }
            | Self::Package { name_index } => writer.write_all(&name_index.to_be_bytes())?,
            Self::String { string_index } => writer.write_all(&string_index.to_be_bytes())?,
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
                writer.write_all(&class_index.to_be_bytes())?;
                writer.write_all(&name_and_type_index.to_be_bytes())?;
            }
            Self::NameAndType {
                name_index,
                descriptor_index,
            } => {
                writer.write_all(&name_index.to_be_bytes())?;
                writer.write_all(&descriptor_index.to_be_bytes())?;
            }
            Self::MethodHandle {
                reference_kind,
                reference_index,
            } => {
                writer.write_all(&reference_kind.to_be_bytes())?;
                writer.write_all(&reference_index.to_be_bytes())?;
            }
            Self::MethodType { descriptor_index } => {
                writer.write_all(&descriptor_index.to_be_bytes())?;
            }
            Self::Dynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            }
            | Self::InvokeDynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => {
                writer.write_all(&bootstrap_method_attr_index.to_be_bytes())?;
                writer.write_all(&name_and_type_index.to_be_bytes())?;
            }
        }
        Ok(())
    }
}
