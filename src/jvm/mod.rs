//! Module containing the APIs for the JVM elements.

pub mod annotation;
pub mod class;
pub mod code;
pub mod field;
pub mod method;
pub mod module;
pub mod parsing;
pub mod class_loader;

pub use parsing::errors::ClassFileParsingError;

/// A [`Result`] type for parsing a class file.
pub type ClassFileParsingResult<T> = Result<T, ClassFileParsingError>;
