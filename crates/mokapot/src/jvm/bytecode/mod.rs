//! JVM class file format parsing.
mod annotation;
mod attribute;
pub(crate) use attribute::Attribute;
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub mod benchmark_support;
pub(super) mod class_element;
pub(super) mod class_file;
pub(super) mod code;
pub mod constant_pool;
pub(crate) mod decoder;
mod field_info;
mod fixed_layout;
mod method_info;
mod module;
mod raw_attributes;

pub use crate::jvm::errors::{ParseError, ParseErrorKind};
use crate::{
    jvm::class::{ConstantPool, Version},
    types::class_name::ClassName,
};

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
    pub current_class_name: ClassName,
}

/// Enables parsing a raw JVM element from a byte slice.
///
/// This trait is implemented by raw JVM elements (without resolving constant pool references) that can be parsed from a byte slice.
pub(crate) trait FromBytecode<'a>: Sized {
    /// Parses an instance of this type from the given slice, advancing it past the parsed element.
    ///
    /// # Errors
    /// Returns a parsing error if the input is truncated or malformed.
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError>;
}
