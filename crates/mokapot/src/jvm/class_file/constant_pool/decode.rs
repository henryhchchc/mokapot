use super::RawEntry;
use crate::jvm::class_file::{Decode, ParseError, decoder::borrow_bytes};

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
            7 => Decode::decode(bytes).map(|name| Self::Class { name }),
            8 => Decode::decode(bytes).map(|string| Self::String { string }),
            9 => {
                let [class, name_and_type] = Decode::decode(bytes)?;
                Ok(Self::FieldRef {
                    class,
                    name_and_type,
                })
            }
            10 => {
                let [class, name_and_type] = Decode::decode(bytes)?;
                Ok(Self::MethodRef {
                    class,
                    name_and_type,
                })
            }
            11 => {
                let [class, name_and_type] = Decode::decode(bytes)?;
                Ok(Self::InterfaceMethodRef {
                    class,
                    name_and_type,
                })
            }
            12 => {
                let [name, descriptor] = Decode::decode(bytes)?;
                Ok(Self::NameAndType { name, descriptor })
            }
            15 => {
                let mut payload = borrow_bytes(bytes, 3)?;
                Ok(Self::MethodHandle {
                    reference_kind: Decode::decode(&mut payload)?,
                    reference: Decode::decode(&mut payload)?,
                })
            }
            16 => Decode::decode(bytes).map(|descriptor| Self::MethodType { descriptor }),
            17 | 18 => {
                let mut payload = borrow_bytes(bytes, 4)?;
                let bootstrap_method_attr_index = Decode::decode(&mut payload)?;
                let name_and_type = Decode::decode(&mut payload)?;
                Ok(if tag == 17 {
                    Self::Dynamic {
                        bootstrap_method_attr_index,
                        name_and_type,
                    }
                } else {
                    Self::InvokeDynamic {
                        bootstrap_method_attr_index,
                        name_and_type,
                    }
                })
            }
            19 => Decode::decode(bytes).map(|name| Self::Module { name }),
            20 => Decode::decode(bytes).map(|name| Self::Package { name }),
            tag => Err(ParseError::malform(format!(
                "Invalid constant pool tag: {tag}"
            ))),
        }
    }
}
