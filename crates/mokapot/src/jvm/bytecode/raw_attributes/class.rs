//! Raw attribute types for class-level and method-level attributes.

use std::io::{self, Read, Write};
use std::result::Result;

use super::super::{
    FromBytecode, ToBytecode, attribute::AttributeInfo, errors::GenerationError,
    reader_utils::BytecodeReader, write_length,
};

pub struct InnerClass {
    pub info_index: u16,
    pub outer_class_info_index: u16,
    pub inner_name_index: u16,
    pub access_flags: u16,
}

impl FromBytecode for InnerClass {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            info_index: reader.decode_value()?,
            outer_class_info_index: reader.decode_value()?,
            inner_name_index: reader.decode_value()?,
            access_flags: reader.decode_value()?,
        })
    }
}

impl ToBytecode for InnerClass {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.info_index.to_be_bytes())?;
        writer.write_all(&self.outer_class_info_index.to_be_bytes())?;
        writer.write_all(&self.inner_name_index.to_be_bytes())?;
        writer.write_all(&self.access_flags.to_be_bytes())?;
        Ok(())
    }
}

pub struct EnclosingMethod {
    pub class_index: u16,
    pub method_index: u16,
}

impl FromBytecode for EnclosingMethod {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            class_index: reader.decode_value()?,
            method_index: reader.decode_value()?,
        })
    }
}

impl ToBytecode for EnclosingMethod {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.class_index.to_be_bytes())?;
        writer.write_all(&self.method_index.to_be_bytes())?;
        Ok(())
    }
}

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

pub struct ParameterInfo {
    pub name_index: u16,
    pub access_flags: u16,
}

impl FromBytecode for ParameterInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            name_index: reader.decode_value()?,
            access_flags: reader.decode_value()?,
        })
    }
}

impl ToBytecode for ParameterInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.name_index.to_be_bytes())?;
        writer.write_all(&self.access_flags.to_be_bytes())?;
        Ok(())
    }
}

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
