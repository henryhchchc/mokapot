//! The parsing logic for the JVM class file format.
mod annotation;
mod attribute;
mod class_file;
mod code;
pub(super) mod constant_pool;
pub(super) mod errors;
mod field_info;
mod jvm_element_parser;
mod method_info;
mod module;
mod reader_utils;

use crate::jvm::{class::Version, constant_pool::ConstantPool};
pub use errors::Error;

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
