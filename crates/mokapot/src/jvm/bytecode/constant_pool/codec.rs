use zerocopy::{
    FromBytes,
    byteorder::big_endian::{F32, F64, I32, I64, U16},
};

use std::io::{self, Read};

use super::RawEntry;
use crate::jvm::bytecode::{FromBytecode, reader::read_bytes};

impl FromBytecode for RawEntry {
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let tag: u8 = u8::read_from_io(&mut reader)?;
        match tag {
            1 => {
                let length: u16 = U16::read_from_io(&mut reader)?.get();
                let bytes = read_bytes(reader, length.into())?;
                Ok(RawEntry::Utf8(bytes))
            }
            3 => I32::read_from_io(reader).map(Self::Integer),
            4 => F32::read_from_io(reader).map(Self::Float),
            5 => I64::read_from_io(reader).map(Self::Long),
            6 => F64::read_from_io(reader).map(Self::Double),
            7 => U16::read_from_io(reader).map(|name_index| Self::Class { name_index }),
            8 => U16::read_from_io(reader).map(|string_index| Self::String { string_index }),
            9 => Ok(Self::FieldRef {
                class_index: U16::read_from_io(&mut reader)?,
                name_and_type_index: U16::read_from_io(&mut reader)?,
            }),
            10 => Ok(Self::MethodRef {
                class_index: U16::read_from_io(&mut reader)?,
                name_and_type_index: U16::read_from_io(&mut reader)?,
            }),
            11 => Ok(Self::InterfaceMethodRef {
                class_index: U16::read_from_io(&mut reader)?,
                name_and_type_index: U16::read_from_io(&mut reader)?,
            }),
            12 => Ok(Self::NameAndType {
                name_index: U16::read_from_io(&mut reader)?,
                descriptor_index: U16::read_from_io(&mut reader)?,
            }),
            15 => Ok(Self::MethodHandle {
                reference_kind: u8::read_from_io(&mut reader)?,
                reference_index: U16::read_from_io(&mut reader)?,
            }),
            16 => Ok(Self::MethodType {
                descriptor_index: U16::read_from_io(&mut reader)?,
            }),
            17 => Ok(Self::Dynamic {
                bootstrap_method_attr_index: U16::read_from_io(&mut reader)?,
                name_and_type_index: U16::read_from_io(&mut reader)?,
            }),
            18 => Ok(Self::InvokeDynamic {
                bootstrap_method_attr_index: U16::read_from_io(&mut reader)?,
                name_and_type_index: U16::read_from_io(&mut reader)?,
            }),
            19 => U16::read_from_io(reader).map(|name_index| Self::Module { name_index }),
            20 => U16::read_from_io(reader).map(|name_index| Self::Package { name_index }),
            tag => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid constant pool tag: {tag}"),
            )),
        }
    }
}
