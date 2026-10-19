//! JVM class file format parsing.

#![expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]

mod annotation;
mod attribute;
mod bootstrap;
pub(super) mod class;
pub(super) mod code;
pub(super) mod constant_pool;
pub(crate) mod decoder;
mod field;
mod fixed_layout;
mod flags;
mod metadata;
mod method;
mod module;
mod nesting;
mod resolve;

use crate::jvm::class::Version;
pub(crate) use crate::jvm::errors::ParseError;

/// Class-wide inputs for resolving raw class-file structures.
#[derive(Debug, Clone)]
pub(super) struct ResolutionContext {
    /// Constants referenced by this class.
    pub constant_pool: constant_pool::ConstantPool,
    /// Format version used for validation.
    pub class_version: Version,
}

pub(crate) use decoder::Decode;
