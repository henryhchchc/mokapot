//! Raw attribute types for class-level and method-level attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    Decode, ParseError,
    attribute::AttributeInfo,
    decoder::{borrow_view, decode_value, parse_repeated},
};
use crate::intrinsics::see_jvm_spec;

#[doc = see_jvm_spec!(4, 7, 6)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct InnerClass {
    pub inner_class_info_index: U16,
    pub outer_class_info_index: U16,
    pub inner_name_index: U16,
    pub inner_class_access_flags: U16,
}

impl FixedLayout for InnerClass {}

#[doc = see_jvm_spec!(4, 7, 7)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct EnclosingMethod {
    pub class_index: U16,
    pub method_index: U16,
}

impl FixedLayout for EnclosingMethod {}

#[doc = see_jvm_spec!(4, 7, 23)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct BootstrapMethod {
    pub bootstrap_method_ref: U16,
    pub num_bootstrap_arguments: U16,
    pub bootstrap_arguments: [U16],
}

impl<'a> Decode<'a> for &'a BootstrapMethod {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        let _: u16 = decode_value(&mut header)?;
        let count: u16 = decode_value(&mut header)?;
        borrow_view::<BootstrapMethod>(bytes, usize::from(count))
    }
}

#[doc = see_jvm_spec!(4, 7, 30)]
pub struct RecordComponentInfo<'a> {
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Box<[&'a AttributeInfo]>,
}

impl<'a> Decode<'a> for RecordComponentInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let name_index = decode_value(bytes)?;
        let descriptor_index = decode_value(bytes)?;
        let attributes_count: u16 = decode_value(bytes)?;
        let attributes = parse_repeated(bytes, attributes_count, <&AttributeInfo>::decode)?;
        Ok(Self {
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_methods_follow_argument_counts() {
        let bytes = [0, 1, 0, 0, 0, 2, 0, 2, 0, 3, 0, 4];
        let mut remaining = bytes.as_slice();
        let empty = <&BootstrapMethod>::decode(&mut remaining).unwrap();
        let no_arguments: &[U16] = &[];
        assert_eq!(empty.bootstrap_method_ref.get(), 1);
        assert_eq!(&empty.bootstrap_arguments, no_arguments);

        let method = <&BootstrapMethod>::decode(&mut remaining).unwrap();
        assert_eq!(method.bootstrap_method_ref.get(), 2);
        assert_eq!(&method.bootstrap_arguments, [U16::new(3), U16::new(4)]);
        assert_eq!(remaining, b"");
    }
}
