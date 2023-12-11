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
mod parsing_context;
mod reader_utils;

pub use parsing_context::ParsingContext;
