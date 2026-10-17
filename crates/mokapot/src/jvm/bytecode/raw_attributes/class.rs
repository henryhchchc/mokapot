//! Raw attribute types for class-level and method-level attributes.

use std::io::{self, Read};

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{FromBytecode, attribute::AttributeInfo, reader::BytecodeReader};
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
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let bootstrap_method_ref = U16::read_from_io(&mut reader)?.get();
        let num_bootstrap_arguments: u16 = U16::read_from_io(&mut reader)?.get();
        let bootstrap_arguments = (0..num_bootstrap_arguments)
            .map(|_| U16::read_from_io(&mut reader).map(u16::from))
            .collect::<io::Result<_>>()?;
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
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let name_index = U16::read_from_io(&mut reader)?.get();
        let descriptor_index = U16::read_from_io(&mut reader)?.get();
        let attributes_count: u16 = U16::read_from_io(&mut reader)?.get();
        let attributes = (0..attributes_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            name_index,
            descriptor_index,
            attributes,
        })
    }
}
