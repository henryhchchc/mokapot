use super::RawEntry;
use crate::jvm::class_file::{
    Decode, ParseError,
    decoder::{borrow_bytes, decode_value},
};

impl<'a> Decode<'a> for RawEntry<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let tag: u8 = decode_value(bytes)?;
        match tag {
            1 => {
                let length: u16 = decode_value(bytes)?;
                let bytes = borrow_bytes(bytes, length.into())?;
                Ok(RawEntry::Utf8(bytes))
            }
            3 => decode_value(bytes).map(Self::Integer),
            4 => decode_value(bytes).map(Self::Float),
            5 => decode_value(bytes).map(Self::Long),
            6 => decode_value(bytes).map(Self::Double),
            7 => decode_value(bytes).map(|name_index| Self::Class { name_index }),
            8 => decode_value(bytes).map(|string_index| Self::String { string_index }),
            9 => Ok(Self::FieldRef {
                class_index: decode_value(bytes)?,
                name_and_type_index: decode_value(bytes)?,
            }),
            10 => Ok(Self::MethodRef {
                class_index: decode_value(bytes)?,
                name_and_type_index: decode_value(bytes)?,
            }),
            11 => Ok(Self::InterfaceMethodRef {
                class_index: decode_value(bytes)?,
                name_and_type_index: decode_value(bytes)?,
            }),
            12 => Ok(Self::NameAndType {
                name_index: decode_value(bytes)?,
                descriptor_index: decode_value(bytes)?,
            }),
            15 => Ok(Self::MethodHandle {
                reference_kind: decode_value(bytes)?,
                reference_index: decode_value(bytes)?,
            }),
            16 => Ok(Self::MethodType {
                descriptor_index: decode_value(bytes)?,
            }),
            17 => Ok(Self::Dynamic {
                bootstrap_method_attr_index: decode_value(bytes)?,
                name_and_type_index: decode_value(bytes)?,
            }),
            18 => Ok(Self::InvokeDynamic {
                bootstrap_method_attr_index: decode_value(bytes)?,
                name_and_type_index: decode_value(bytes)?,
            }),
            19 => decode_value(bytes).map(|name_index| Self::Module { name_index }),
            20 => decode_value(bytes).map(|name_index| Self::Package { name_index }),
            tag => Err(ParseError::malform(format!(
                "Invalid constant pool tag: {tag}"
            ))),
        }
    }
}
