use zerocopy::{FromBytes, Immutable, Unaligned, byteorder::big_endian::U16};

use super::RawEntry;
use crate::jvm::class_file::{
    Decode, ParseError, decoder::borrow_bytes, fixed_layout::FixedLayout,
};

#[derive(FromBytes, Immutable, Unaligned)]
#[repr(C)]
struct MethodHandlePayload {
    reference_kind: u8,
    reference_index: U16,
}

impl FixedLayout for MethodHandlePayload {}

impl<'a> Decode<'a> for RawEntry<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let tag = u8::decode(bytes)?;
        match tag {
            1 => {
                let length = u16::decode(bytes)?;
                let bytes = borrow_bytes(bytes, length.into())?;
                Ok(RawEntry::Utf8(bytes))
            }
            3 => Decode::decode(bytes).map(Self::Integer),
            4 => Decode::decode(bytes).map(Self::Float),
            5 => Decode::decode(bytes).map(Self::Long),
            6 => Decode::decode(bytes).map(Self::Double),
            7 => Decode::decode(bytes).map(|name_index| Self::Class { name_index }),
            8 => Decode::decode(bytes).map(|string_index| Self::String { string_index }),
            9 => {
                <[U16; 2]>::decode(bytes).map(|[class_index, name_and_type_index]| Self::FieldRef {
                    class_index,
                    name_and_type_index,
                })
            }
            10 => <[U16; 2]>::decode(bytes).map(|[class_index, name_and_type_index]| {
                Self::MethodRef {
                    class_index,
                    name_and_type_index,
                }
            }),
            11 => <[U16; 2]>::decode(bytes).map(|[class_index, name_and_type_index]| {
                Self::InterfaceMethodRef {
                    class_index,
                    name_and_type_index,
                }
            }),
            12 => {
                <[U16; 2]>::decode(bytes).map(|[name_index, descriptor_index]| Self::NameAndType {
                    name_index,
                    descriptor_index,
                })
            }
            15 => MethodHandlePayload::decode(bytes).map(|payload| Self::MethodHandle {
                reference_kind: payload.reference_kind,
                reference_index: payload.reference_index,
            }),
            16 => Ok(Self::MethodType {
                descriptor_index: Decode::decode(bytes)?,
            }),
            17 => <[U16; 2]>::decode(bytes).map(
                |[bootstrap_method_attr_index, name_and_type_index]| Self::Dynamic {
                    bootstrap_method_attr_index,
                    name_and_type_index,
                },
            ),
            18 => <[U16; 2]>::decode(bytes).map(
                |[bootstrap_method_attr_index, name_and_type_index]| Self::InvokeDynamic {
                    bootstrap_method_attr_index,
                    name_and_type_index,
                },
            ),
            19 => Decode::decode(bytes).map(|name_index| Self::Module { name_index }),
            20 => Decode::decode(bytes).map(|name_index| Self::Package { name_index }),
            tag => Err(ParseError::malform(format!(
                "Invalid constant pool tag: {tag}"
            ))),
        }
    }
}
