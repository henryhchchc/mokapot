use crate::elements::class::ClassVersion;

use super::constant_pool::ConstantPool;

#[derive(Debug)]
pub(crate) struct ParsingContext {
    pub constant_pool: ConstantPool,
    pub class_version: ClassVersion,
    pub current_class_binary_name: String,
}
