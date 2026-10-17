//! JVM class file format parsing.
mod annotation;
mod attribute;
pub(super) mod class;
pub(super) mod code;
pub(super) mod constant_pool;
pub(crate) mod decoder;
mod field;
mod fixed_layout;
mod flags;
mod method;
mod module;
mod resolve;

pub(crate) use crate::jvm::errors::ParseError;
use crate::{jvm::class::Version, types::class_name::ClassName};

/// Class-wide inputs for resolving raw class-file structures.
#[derive(Debug, Clone)]
pub(super) struct ResolutionContext {
    /// Constants referenced by this class.
    pub constant_pool: constant_pool::ConstantPool,
    /// Format version used for validation.
    pub class_version: Version,
    /// Internal name of this class.
    pub current_class_name: ClassName,
}

pub(crate) use decoder::Decode;
