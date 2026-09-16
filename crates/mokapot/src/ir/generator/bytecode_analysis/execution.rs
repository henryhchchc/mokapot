use std::collections::BTreeMap;

use super::{Executor, Value};
use crate::{
    ir::generator::{
        bytecode_analysis::jvm::Frame,
        error::{Error, MalformedBytecode},
        identity::SsaValueId,
    },
    jvm::{Method, method},
};

impl<'method> Executor<'method> {
    pub(super) fn for_method(method: &'method Method) -> Result<Self, Error> {
        let body = method.body.as_ref().ok_or(Error::NoMethodBody)?;
        body.instructions
            .entry_point()
            .ok_or_else(|| Error::malformed(None, MalformedBytecode::MissingEntry))?;

        let mut value_id_allocator = ValueIdAllocator::default();
        let receiver_value = (!method.access_flags.contains(method::AccessFlags::STATIC))
            .then(|| value_id_allocator.new_value_id())
            .transpose()?;
        let parameter_values: Vec<SsaValueId> = method
            .descriptor
            .parameters_types
            .iter()
            .map(|_| value_id_allocator.new_value_id())
            .collect::<Result<_, _>>()?;
        let frame_parameters = parameter_values
            .iter()
            .copied()
            .map(Value::Ssa)
            .collect::<Vec<_>>();
        let initial_frame = Frame::for_method_entry(
            &method.descriptor,
            body.max_locals,
            body.max_stack,
            receiver_value.map(Value::Ssa),
            &frame_parameters,
        )?;
        Ok(Self {
            body,
            definition_ids: BTreeMap::new(),
            value_id_allocator,
            receiver_value,
            parameter_values,
            initial_frame,
        })
    }

    pub(super) fn new_value_id(&mut self) -> Result<SsaValueId, Error> {
        self.value_id_allocator.new_value_id()
    }

    pub(super) fn definition_id_at(
        &mut self,
        pc: crate::jvm::code::ProgramCounter,
    ) -> Result<SsaValueId, Error> {
        if let Some(&id) = self.definition_ids.get(&pc) {
            return Ok(id);
        }
        let id = self.new_value_id()?;
        self.definition_ids.insert(pc, id);
        Ok(id)
    }
}

#[derive(Debug, Default)]
pub(super) struct ValueIdAllocator {
    next_value_idx: u32,
}

impl ValueIdAllocator {
    pub(super) fn new_value_id(&mut self) -> Result<SsaValueId, Error> {
        let id = SsaValueId::new(self.next_value_idx);
        self.next_value_idx = self
            .next_value_idx
            .checked_add(1)
            .ok_or_else(|| Error::internal("the scalar value identity space is exhausted"))?;
        Ok(id)
    }
}
