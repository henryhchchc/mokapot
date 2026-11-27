//! Raw attribute types for class-level and method-level attributes.

use zerocopy::{Immutable, KnownLayout, TryFromBytes, Unaligned, byteorder::big_endian::U16};

use super::super::{
    Decode, ParseError,
    attribute::AttributeInfo,
    decoder::{borrow_bytes, parse_repeated, try_borrow_view},
};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{ConstantPoolIndex, constant_pool::OptionalConstantPoolIndex},
};

/// Shared wire layout of `field_info` and `method_info`.
#[doc = see_jvm_spec!(4, 5)]
#[doc = see_jvm_spec!(4, 6)]
#[derive(Debug)]
pub struct MemberInfo<'a> {
    pub access_flags: u16,
    pub name: ConstantPoolIndex,
    pub descriptor: ConstantPoolIndex,
    pub attributes: Box<[&'a AttributeInfo]>,
}

impl<'a> Decode<'a> for MemberInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = borrow_bytes(bytes, 8)?;
        Ok(Self {
            access_flags: Decode::decode(&mut header)?,
            name: Decode::decode(&mut header)?,
            descriptor: Decode::decode(&mut header)?,
            attributes: parse_repeated(bytes, u16::decode(&mut header)?, <&AttributeInfo>::decode)?,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 6)]
#[derive(Clone, Copy, TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct InnerClass {
    pub inner_class_info: ConstantPoolIndex,
    pub outer_class_info: OptionalConstantPoolIndex,
    pub inner_name: OptionalConstantPoolIndex,
    pub inner_class_access_flags: U16,
}

#[doc = see_jvm_spec!(4, 7, 7)]
#[derive(Clone, Copy)]
pub struct EnclosingMethod {
    pub class: ConstantPoolIndex,
    pub method: Option<ConstantPoolIndex>,
}

impl<'a> Decode<'a> for EnclosingMethod {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut fields = borrow_bytes(bytes, 4)?;
        Ok(Self {
            class: Decode::decode(&mut fields)?,
            method: Decode::decode(&mut fields)?,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 23)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct BootstrapMethod {
    pub bootstrap_method_ref: ConstantPoolIndex,
    pub num_bootstrap_arguments: U16,
    pub bootstrap_arguments: [ConstantPoolIndex],
}

impl<'a> Decode<'a> for &'a BootstrapMethod {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        u16::decode(&mut header)?;
        let count = usize::from(u16::decode(&mut header)?);
        try_borrow_view(bytes, count)
    }
}

#[doc = see_jvm_spec!(4, 7, 30)]
pub struct RecordComponentInfo<'a> {
    pub name: ConstantPoolIndex,
    pub descriptor: ConstantPoolIndex,
    pub attributes: Box<[&'a AttributeInfo]>,
}

impl<'a> Decode<'a> for RecordComponentInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let name = Decode::decode(bytes)?;
        let descriptor = Decode::decode(bytes)?;
        let attributes_count = u16::decode(bytes)?;
        let attributes = parse_repeated(bytes, attributes_count, <&AttributeInfo>::decode)?;
        Ok(Self {
            name,
            descriptor,
            attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::{class_file::attribute::raw::InnerClasses, errors::ParseErrorKind};

    #[test]
    fn inner_classes_borrow_optional_indices() {
        let mut bytes = [0, 1, 0, 1, 0, 0, 0x12, 0x34, 0, 0].as_slice();
        let class = <&InnerClasses>::decode(&mut bytes).unwrap().classes[0];
        assert_eq!(class.outer_class_info.get(), None);
        assert_eq!(
            class.inner_name.get().map(ConstantPoolIndex::get),
            Some(0x1234)
        );
    }

    #[test]
    fn bootstrap_methods_follow_argument_counts() {
        let bytes = [0, 1, 0, 0, 0, 2, 0, 2, 0, 3, 0, 4];
        let mut remaining = bytes.as_slice();
        let empty = <&BootstrapMethod>::decode(&mut remaining).unwrap();
        assert_eq!(empty.bootstrap_method_ref.get(), 1);
        assert_eq!(empty.bootstrap_arguments, []);

        let method = <&BootstrapMethod>::decode(&mut remaining).unwrap();
        assert_eq!(method.bootstrap_method_ref.get(), 2);
        let arguments = [3, 4].map(|index| ConstantPoolIndex::new(index).unwrap());
        assert_eq!(method.bootstrap_arguments, arguments);
        assert_eq!(remaining, b"");
    }

    #[test]
    fn bootstrap_methods_reject_zero_indices_and_truncated_arguments() {
        let zero_index = ParseErrorKind::InvalidConstantPoolIndex { index: 0 };
        for (bytes, expected) in [
            (&[0, 0, 0, 0][..], zero_index),
            (&[0, 1, 0, 1, 0, 0][..], zero_index),
            (
                &[0, 1, 0, 1][..],
                ParseErrorKind::Truncated {
                    requested: 6,
                    available: 4,
                },
            ),
        ] {
            let mut remaining = bytes;
            let error = <&BootstrapMethod>::decode(&mut remaining).unwrap_err();
            assert_eq!(error.kind(), expected);
            assert_eq!(remaining, bytes);
        }
    }
}
