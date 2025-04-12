//! The parsing logic for the JVM class file format.
mod annotation;
mod attribute;
pub(super) mod class_file;
mod code;
pub(super) mod constant_pool;
pub(super) mod errors;
mod field_info;
mod jvm_element_parser;
mod method_info;
mod module;
mod raw_attributes;
mod reader_utils;

use std::{
    io::{self, Read, Write},
    num::TryFromIntError,
};

pub use errors::ParsingError;
use errors::ToWriterError;
use num_traits::ToBytes;

use crate::jvm::class::{ConstantPool, Version};

/// Context used to parse a class file.
#[derive(Debug, Clone)]
pub struct ParsingContext {
    /// The constant pool of the class file.
    pub constant_pool: ConstantPool,
    /// The version of the class file being parsed.
    pub class_version: Version,
    /// The binary name of the class being parsed.
    pub current_class_binary_name: String,
}

trait FromReader {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized;
}

/// Trait for writing a Raw JVM element to a writer.
pub trait ToWriter {
    /// Writes the Raw JVM element to the given writer.
    ///
    /// # Errors
    /// This function will only forward the error returned by the underlying writer.
    fn to_writer<W: Write>(&self, writer: &mut W) -> Result<(), ToWriterError>;
}

fn write_length<Len>(writer: &mut impl Write, length: usize) -> Result<(), ToWriterError>
where
    usize: TryInto<Len, Error = TryFromIntError>,
    Len: ToBytes,
    <Len as ToBytes>::Bytes: IntoIterator<Item = u8>,
{
    let length = length.try_into()?;
    writer.write_all(length.to_be_bytes().as_ref())?;
    Ok(())
}
