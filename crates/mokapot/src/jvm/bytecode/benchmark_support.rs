//! Optional access to the raw class parser for benchmarks.

use super::{ParseError, class_file::ClassFile};

/// Parse a class file without resolving its raw elements into the public model.
///
/// # Errors
/// Returns class-file parsing errors.
pub fn raw_class(bytes: &[u8]) -> Result<impl Sized + '_, ParseError> {
    ClassFile::parse_complete(bytes)
}
