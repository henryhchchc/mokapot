//! Raw attribute types for class-level and method-level attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    Decode, ParseError,
    attribute::AttributeInfo,
    decoder::{borrow_slice, decode_value, parse_repeated},
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

impl<'a> Decode<'a> for BootstrapMethod<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let bootstrap_method_ref = decode_value(bytes)?;
        let num_bootstrap_arguments: u16 = decode_value(bytes)?;
        let bootstrap_arguments = borrow_slice(bytes, usize::from(num_bootstrap_arguments))?;
        Ok(Self {
            bootstrap_method_ref,
            bootstrap_arguments,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 30)]
pub struct RecordComponentInfo<'a> {
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Box<[AttributeInfo<'a>]>,
}

impl<'a> Decode<'a> for RecordComponentInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let name_index = decode_value(bytes)?;
        let descriptor_index = decode_value(bytes)?;
        let attributes_count: u16 = decode_value(bytes)?;
        let attributes = parse_repeated(bytes, attributes_count, AttributeInfo::decode)?;
        Ok(Self {
            name_index,
            descriptor_index,
            attributes,
        })
    }
}
