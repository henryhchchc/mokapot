//! Raw attribute types for class-level and method-level attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    FromBytecode, ParseError,
    attribute::AttributeInfo,
    decoder::{BytecodeSlice, read_value},
};
use crate::intrinsics::see_jvm_spec;

#[doc = see_jvm_spec!(4, 7, 6)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
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
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct EnclosingMethod {
    pub class_index: U16,
    pub method_index: U16,
}

impl FixedLayout for EnclosingMethod {}

#[doc = see_jvm_spec!(4, 7, 23)]
pub struct BootstrapMethod {
    pub bootstrap_method_ref: u16,
    pub bootstrap_arguments: Box<[u16]>,
}

impl FromBytecode for BootstrapMethod {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let bootstrap_method_ref = read_value::<U16>(bytes)?.get();
        let num_bootstrap_arguments: u16 = read_value::<U16>(bytes)?.get();
        let bootstrap_arguments = (0..num_bootstrap_arguments)
            .map(|_| read_value::<U16>(bytes).map(u16::from))
            .collect::<Result<_, ParseError>>()?;
        Ok(Self {
            bootstrap_method_ref,
            bootstrap_arguments,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ParameterInfo {
    pub name_index: U16,
    pub access_flags: U16,
}

impl FixedLayout for ParameterInfo {}

#[doc = see_jvm_spec!(4, 7, 30)]
pub struct RecordComponentInfo {
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Box<[AttributeInfo]>,
}

impl FromBytecode for RecordComponentInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
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
