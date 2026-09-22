use std::collections::HashMap;

use super::Frame;
use crate::{
    ir::{
        BlockId, IdAllocator, ValueId,
        generator::{controlflow::Cfg, error::Error},
    },
    jvm::{code::ProgramCounter, method},
};

pub(super) struct ValueContext {
    definition_ids: HashMap<ProgramCounter, ValueId>,
    caught_exceptions: HashMap<BlockId, ValueId>,
    value_id_allocator: IdAllocator<ValueId>,
    receiver_value: Option<ValueId>,
    parameter_values: Vec<ValueId>,
}

impl ValueContext {
    pub(super) fn for_cfg(cfg: &Cfg<'_>) -> Result<(Self, Frame), Error> {
        let method = cfg.method();
        let body = cfg.body();

        let mut value_id_allocator = IdAllocator::default();
        let receiver_value = (!method.access_flags.contains(method::AccessFlags::STATIC))
            .then(|| value_id_allocator.new_id());
        let parameter_values: Vec<ValueId> = method
            .descriptor
            .parameters_types
            .iter()
            .map(|_| value_id_allocator.new_id())
            .collect();
        let initial_frame = Frame::for_method_entry(
            &method.descriptor,
            body.max_locals,
            body.max_stack,
            receiver_value,
            &parameter_values,
        )?;
        let values = Self {
            definition_ids: HashMap::new(),
            caught_exceptions: HashMap::new(),
            value_id_allocator,
            receiver_value,
            parameter_values,
        };
        Ok((values, initial_frame))
    }

    pub(super) fn fresh(&mut self) -> ValueId {
        self.value_id_allocator.new_id()
    }

    /// Returns the single value identity associated with a handler entry.
    pub(super) fn caught_exception(&mut self, handler: BlockId) -> ValueId {
        *self
            .caught_exceptions
            .entry(handler)
            .or_insert_with(|| self.value_id_allocator.new_id())
    }

    pub(super) fn into_method_values(self) -> (Option<ValueId>, Vec<ValueId>) {
        (self.receiver_value, self.parameter_values)
    }

    pub(super) fn definition_at(&mut self, pc: ProgramCounter) -> ValueId {
        *self
            .definition_ids
            .entry(pc)
            .or_insert_with(|| self.value_id_allocator.new_id())
    }
}
