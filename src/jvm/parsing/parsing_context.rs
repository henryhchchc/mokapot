use crate::jvm::{class::ClassVersion, constant_pool::ConstantPool};

/// Context used to parse a class file.
#[derive(Debug, Clone)]
pub struct ParsingContext {
    /// The constant pool of the class file.
    pub constant_pool: ConstantPool,
    /// The version of the class file being parsed.
    pub class_version: ClassVersion,
    /// The binary name of the class being parsed.
    pub current_class_binary_name: String,
}
