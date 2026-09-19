use std::collections::HashMap;

use super::{EntrySlots, Frame, Position};
use crate::{
    ir::{
        IdAllocator, ValueId,
        generator::{bytecode_cfg::Cfg, error::Error},
    },
    jvm::{code::ProgramCounter, method},
};

pub(super) struct ValueContext {
    definition_ids: HashMap<ProgramCounter, ValueId>,
    value_id_allocator: IdAllocator<ValueId>,
    pub(super) receiver_value: Option<ValueId>,
    pub(super) parameter_values: Vec<ValueId>,
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
        let (initial_frame, entry_slots) = Frame::for_method_entry(
            &method.descriptor,
            body.max_locals,
            body.max_stack,
            receiver_value,
            &parameter_values,
        )?;
        Self::debug_assert_entry_frame(
            &initial_frame,
            &entry_slots,
            receiver_value,
            &parameter_values,
        );
        let values = Self {
            definition_ids: HashMap::new(),
            value_id_allocator,
            receiver_value,
            parameter_values,
        };
        Ok((values, initial_frame))
    }

    /// Checks the entry-frame convention: parameters follow `this` in descriptor
    /// order, with a category-2 parameter occupying two slots, so the identity
    /// allocated to a parameter is the value held in that parameter's slot.
    fn debug_assert_entry_frame(
        frame: &Frame,
        entry_slots: &EntrySlots,
        receiver_value: Option<ValueId>,
        parameter_values: &[ValueId],
    ) {
        debug_assert_eq!(
            entry_slots.parameters.len(),
            parameter_values.len(),
            "every allocated parameter has an entry slot"
        );
        debug_assert_eq!(
            entry_slots
                .this
                .and_then(|slot| frame.value_at(Position::Local(slot.into())).copied()),
            receiver_value,
            "the receiver identity must occupy its entry slot"
        );
        for (&slot, &value) in entry_slots.parameters.iter().zip(parameter_values) {
            debug_assert_eq!(
                frame.value_at(Position::Local(slot.into())).copied(),
                Some(value),
                "a parameter identity must occupy its entry slot"
            );
        }
    }

    pub(super) fn fresh(&mut self) -> ValueId {
        self.value_id_allocator.new_id()
    }

    pub(super) fn definition_at(&mut self, pc: ProgramCounter) -> ValueId {
        if let Some(&id) = self.definition_ids.get(&pc) {
            return id;
        }
        let id = self.fresh();
        self.definition_ids.insert(pc, id);
        id
    }
}
