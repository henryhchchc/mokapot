//! JVM elements, such as classes, methods, fields, and annotations.

pub mod annotation;
pub mod attribute;
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub mod benchmark_support;
pub mod class;
mod class_file;
pub mod class_loader;
pub mod code;
pub mod errors;
pub mod field;
pub mod method;
pub mod module;
pub mod references;

mod constant_pool;
mod constant_value;
mod debug;
pub(crate) use constant_pool::ConstantPool;
pub(crate) use debug::DebugList;

mod string;
pub use annotation::{
    Annotation, Annotations, ParameterAnnotations, TypeAnnotation, TypeAnnotations,
};
pub use attribute::Attribute;
pub use class::Class;
pub use class_loader::ClassLoader;
pub use constant_value::{ConstantVal, ConstantValue};
pub use field::Field;
pub use method::Method;
pub use module::Module;
pub use string::{Cesu8Error, FromCesu8Error, JavaStr, JavaStrDecodeError, JavaString};
