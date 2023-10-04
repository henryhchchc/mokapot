pub mod annotation;
pub(crate) mod class;
pub(crate) mod class_parser;
pub(crate) mod field;
pub(crate) mod instruction;
pub(crate) mod method;
pub mod module;
pub(crate) mod parsing;
pub mod pc;
pub(crate) mod references;

pub use class::*;
pub use class_parser::ClassParser;
pub use instruction::*;
pub use method::*;
pub use references::*;
