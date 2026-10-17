//! Raw attribute types for class-level and method-level attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    FromBytecode, ParseError,
    attribute::AttributeInfo,
    decoder::{BytecodeSlice, read_slice, read_value},
};
use crate::intrinsics::see_jvm_spec;

#[doc = see_jvm_spec!(4, 7, 6)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
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
pub struct BootstrapMethod<'a> {
    pub bootstrap_method_ref: u16,
    pub bootstrap_arguments: &'a [U16],
}

impl<'a> FromBytecode<'a> for BootstrapMethod<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let bootstrap_method_ref = read_value::<U16>(bytes)?.get();
        let num_bootstrap_arguments: u16 = read_value::<U16>(bytes)?.get();
        let bootstrap_arguments = read_slice(bytes, usize::from(num_bootstrap_arguments))?;
        Ok(Self {
            bootstrap_method_ref,
            bootstrap_arguments,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ParameterInfo {
    pub name_index: U16,
    pub access_flags: U16,
}

impl FixedLayout for ParameterInfo {}

#[doc = see_jvm_spec!(4, 7, 30)]
pub struct RecordComponentInfo<'a> {
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Box<[AttributeInfo<'a>]>,
}

impl<'a> FromBytecode<'a> for RecordComponentInfo<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let name_index = read_value::<U16>(bytes)?.get();
        let descriptor_index = read_value::<U16>(bytes)?.get();
        let attributes_count: u16 = read_value::<U16>(bytes)?.get();
        let attributes = (0..attributes_count)
            .map(|_| bytes.decode_value())
            .collect::<Result<_, ParseError>>()?;
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
    fn bootstrap_arguments_borrow_their_input() {
        let bytes = [0, 3, 0, 2, 0, 4, 0, 5];
        let mut remaining = bytes.as_slice();
        let method = BootstrapMethod::from_bytes(&mut remaining).unwrap();
        assert_eq!(remaining, []);
        assert_eq!(method.bootstrap_arguments[0].get(), 4);
        assert_eq!(method.bootstrap_arguments[1].get(), 5);
        assert!(std::ptr::eq(
            method.bootstrap_arguments.as_ptr().cast::<u8>(),
            bytes[4..].as_ptr()
        ));
    }
}
