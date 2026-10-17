use zerocopy::byteorder::big_endian::{F32, F64, I32, I64, U16};

use super::RawEntry;
use crate::jvm::bytecode::{
    FromBytecode, ParseError,
    decoder::{read_bytes, read_value},
};

impl<'a> FromBytecode<'a> for RawEntry<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let tag: u8 = read_value::<u8>(bytes)?;
        match tag {
            1 => {
                let length: u16 = read_value::<U16>(bytes)?.get();
                let bytes = read_bytes(bytes, length.into())?;
                Ok(RawEntry::Utf8(bytes))
            }
            3 => read_value::<I32>(bytes).map(Self::Integer),
            4 => read_value::<F32>(bytes).map(Self::Float),
            5 => read_value::<I64>(bytes).map(Self::Long),
            6 => read_value::<F64>(bytes).map(Self::Double),
            7 => read_value::<U16>(bytes).map(|name_index| Self::Class { name_index }),
            8 => read_value::<U16>(bytes).map(|string_index| Self::String { string_index }),
            9 => Ok(Self::FieldRef {
                class_index: read_value::<U16>(bytes)?,
                name_and_type_index: read_value::<U16>(bytes)?,
            }),
            10 => Ok(Self::MethodRef {
                class_index: read_value::<U16>(bytes)?,
                name_and_type_index: read_value::<U16>(bytes)?,
            }),
            11 => Ok(Self::InterfaceMethodRef {
                class_index: read_value::<U16>(bytes)?,
                name_and_type_index: read_value::<U16>(bytes)?,
            }),
            12 => Ok(Self::NameAndType {
                name_index: read_value::<U16>(bytes)?,
                descriptor_index: read_value::<U16>(bytes)?,
            }),
            15 => Ok(Self::MethodHandle {
                reference_kind: read_value::<u8>(bytes)?,
                reference_index: read_value::<U16>(bytes)?,
            }),
            16 => Ok(Self::MethodType {
                descriptor_index: read_value::<U16>(bytes)?,
            }),
            17 => Ok(Self::Dynamic {
                bootstrap_method_attr_index: read_value::<U16>(bytes)?,
                name_and_type_index: read_value::<U16>(bytes)?,
            }),
            18 => Ok(Self::InvokeDynamic {
                bootstrap_method_attr_index: read_value::<U16>(bytes)?,
                name_and_type_index: read_value::<U16>(bytes)?,
            }),
            19 => read_value::<U16>(bytes).map(|name_index| Self::Module { name_index }),
            20 => read_value::<U16>(bytes).map(|name_index| Self::Package { name_index }),
            tag => Err(ParseError::malform(format!(
                "Invalid constant pool tag: {tag}"
            ))),
        }
    }
}
