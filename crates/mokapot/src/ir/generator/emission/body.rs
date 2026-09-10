use std::collections::BTreeMap;

use crate::{
    ir::{BasicBlock, BlockId, MokaIRMethod, SourceMap, ValueDefinition, ValueId},
    jvm::Method,
};

/// The completed generated body before JVM method metadata is attached.
#[derive(Debug)]
pub(in crate::ir::generator) struct GeneratedBody {
    pub(in crate::ir::generator) entry: BlockId,
    pub(in crate::ir::generator) blocks: Vec<BasicBlock>,
    pub(in crate::ir::generator) source_map: SourceMap,
    pub(in crate::ir::generator) this_value: Option<ValueId>,
    pub(in crate::ir::generator) parameter_values: Vec<ValueId>,
    pub(in crate::ir::generator) caught_exceptions: BTreeMap<BlockId, ValueId>,
    pub(in crate::ir::generator) value_definitions: Vec<ValueDefinition>,
}

impl GeneratedBody {
    /// Attaches JVM method metadata to this completed generated body.
    pub(in crate::ir::generator) fn into_method(self, method: &Method) -> MokaIRMethod {
        MokaIRMethod::new(
            method.access_flags,
            method.name.clone(),
            method.descriptor.clone(),
            method.owner.clone(),
            self.entry,
            self.blocks,
            self.source_map,
            self.this_value,
            self.parameter_values,
            self.caught_exceptions,
            self.value_definitions,
        )
    }
}
