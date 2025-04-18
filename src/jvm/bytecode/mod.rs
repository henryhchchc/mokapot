//! JVM class file format parsing and writing functionality.
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

pub use errors::{ParsingError, ParsingErrorKind, ToWriterError};
use num_traits::ToBytes;

use crate::jvm::class::{ConstantPool, Version};

/// Maintains context for parsing a JVM class file.
///
/// This structure holds essential information needed during the class file parsing process,
/// including the constant pool, class version, and binary name of the class. This context
/// is passed through various parsing functions to provide access to shared data and maintain
/// parsing state.
#[derive(Debug, Clone)]
pub struct ParsingContext {
    /// The constant pool of the class file, containing all constant entries referenced by the class.
    pub constant_pool: ConstantPool,
    /// The version of the class file being parsed, indicating JVM compatibility requirements.
    pub class_version: Version,
    /// The binary name of the class being parsed (e.g., "java/lang/String").
    pub current_class_binary_name: String,
}

/// Enables parsing a raw JVM element from a binary stream.
///
/// This trait is implemented by raw JVM elements (without resolving constant pool references) that can be parsed from a binary input stream.
trait FromReader {
    /// Parses an instance of this type from the given reader.
    ///
    /// # Errors
    /// Returns an `io::Error` if reading from the stream fails or if the data
    /// is not in the expected format.
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized;
}

/// Enables writing a raw JVM element to a binary stream.
///
/// This trait is implemented by raw JVM elements (after putting all the constants in the constant pool) that can be serialized to a class file format.
/// It provides a standardized way to write JVM elements back to binary form.
trait ToWriter {
    /// Writes this element to the given writer in JVM class file format.
    ///
    /// # Errors
    /// Returns a `ToWriterError` if writing to the stream fails or if the data
    /// cannot be properly serialized (e.g., if a numeric value is out of range).
    fn to_writer<W>(&self, writer: &mut W) -> Result<(), ToWriterError>
    where
        W: Write + ?Sized;
}

/// Writes a length value in the appropriate binary format for JVM class files.
///
/// This utility function handles the conversion of Rust `usize` values to the appropriate
/// fixed-width type used in the class file format, writing the result in big-endian order.
///
/// # Type Parameters
/// * `Len` - The target numeric type for the length (e.g., u16 for method parameter counts)
///
/// # Errors
/// Returns a `ToWriterError` if:
/// - The length value cannot fit in the target type
/// - Writing to the output stream fails
fn write_length<Len>(writer: &mut (impl Write + ?Sized), length: usize) -> Result<(), ToWriterError>
where
    usize: TryInto<Len, Error = TryFromIntError>,
    Len: ToBytes,
    <Len as ToBytes>::Bytes: IntoIterator<Item = u8>,
{
    let length = length.try_into()?;
    writer.write_all(length.to_be_bytes().as_ref())?;
    Ok(())
}
