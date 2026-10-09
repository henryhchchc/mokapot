//! Optional access to the raw class parser for benchmarks.

use super::{FromBytecode, ParseError, class_file::ClassFile};

/// Parse a class file without resolving its raw elements into the public model.
///
/// # Errors
/// Returns class-file parsing or I/O errors.
pub fn raw_class(mut bytes: &[u8]) -> Result<impl Sized, ParseError> {
    Ok(ClassFile::from_reader(&mut bytes)?)
}
