//! Raw attribute types for class-level and method-level attributes.

use std::{
    io::{self, Read, Write},
    result::Result,
};

use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, byteorder::big_endian::U16,
};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    FromBytecode, GenerationError, ToBytecode, attribute::AttributeInfo, reader::BytecodeReader,
    write_length,
};
use crate::intrinsics::see_jvm_spec;

#[doc = see_jvm_spec!(4, 7, 6)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct InnerClass {
    pub info_index: U16,
    pub outer_class_info_index: U16,
    pub inner_name_index: U16,
    pub access_flags: U16,
}

impl FixedLayout for InnerClass {}

#[doc = see_jvm_spec!(4, 7, 7)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct EnclosingMethod {
    pub class_index: U16,
    pub method_index: U16,
}

impl FixedLayout for EnclosingMethod {}

#[doc = see_jvm_spec!(4, 7, 23)]
pub struct BootstrapMethod {
    pub method_ref_idx: u16,
    pub arguments: Vec<u16>,
}

impl FromBytecode for BootstrapMethod {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let method_ref_idx = reader.decode_value()?;
        let num_arguments: u16 = reader.decode_value()?;
        let arguments = (0..num_arguments)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            method_ref_idx,
            arguments,
        })
    }
}

impl ToBytecode for BootstrapMethod {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.method_ref_idx.to_be_bytes())?;
        write_length::<u16>(writer, self.arguments.len())?;
        for argument in &self.arguments {
            writer.write_all(&argument.to_be_bytes())?;
        }
        Ok(())
    }
}

#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
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
    pub attributes: Vec<AttributeInfo>,
}

impl FromBytecode for RecordComponentInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let name_index = reader.decode_value()?;
        let descriptor_index = reader.decode_value()?;
        let attributes_count: u16 = reader.decode_value()?;
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

impl ToBytecode for RecordComponentInfo {
    fn to_writer<W>(&self, writer: &mut W) -> Result<(), GenerationError>
    where
        W: Write + ?Sized,
    {
        writer.write_all(&self.name_index.to_be_bytes())?;
        writer.write_all(&self.descriptor_index.to_be_bytes())?;
        self.attributes.to_writer(writer)?;
        Ok(())
    }
}
