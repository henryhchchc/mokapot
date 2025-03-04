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
    io::{self, Write},
    num::TryFromIntError,
};

use crate::jvm::class::{ConstantPool, Version};
use derive_more::Display;
pub use errors::Error;
use num_traits::ToBytes;

/// Context used to parse a class file.
#[derive(Debug, Clone)]
pub struct Context {
    /// The constant pool of the class file.
    pub constant_pool: ConstantPool,
    /// The version of the class file being parsed.
    pub class_version: Version,
    /// The binary name of the class being parsed.
    pub current_class_binary_name: String,
}

/// Trait for writing a Raw JVM element to a writer.
pub trait ToWriter {
    /// Writes the Raw JVM element to the given writer.
    ///
    /// # Errors
    /// This function will only forward the error returned by the underlying writer.
    fn to_writer<W: Write>(&self, writer: &mut W) -> Result<(), ToWriterError>;
}

/// Error that can occur when writing a Raw JVM element to a writer.
#[derive(Debug, Display, thiserror::Error)]
pub enum ToWriterError {
    /// Error from the underlying writer.
    IO(#[from] io::Error),
    /// A list of elements is too long that it exceeds the data type for the length.
    ListTooLong(#[from] TryFromIntError),
}

pub(in crate::jvm::parsing) fn write_length<L, W>(
    writer: &mut W,
    length: usize,
) -> Result<(), ToWriterError>
where
    W: Write,
    L: TryFrom<usize, Error = TryFromIntError> + ToBytes,
{
    let length = L::try_from(length)?;
    writer.write_all(length.to_be_bytes().as_ref())?;
    Ok(())
}
