use super::{ValueCategory, error::Error};
use crate::{ir::ValueId, types::method_descriptor::MethodDescriptor};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct LocalValue {
    value: ValueId,
    category: ValueCategory,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
enum LocalSlot {
    Value(LocalValue),
    Reserved,
    Unset,
    Unavailable,
}

impl LocalSlot {
    const fn value(&self) -> Option<&ValueId> {
        match self {
            Self::Value(value) => Some(&value.value),
            Self::Reserved | Self::Unset | Self::Unavailable => None,
        }
    }

    fn merge_from_with<E>(
        &mut self,
        other: Self,
        join_values: impl FnOnce(&mut ValueId, ValueId) -> Result<(), E>,
    ) -> Result<(), E> {
        use LocalSlot::{Reserved, Unavailable, Unset, Value};
        match (self, other) {
            (Value(lhs), Value(rhs)) if lhs.category == rhs.category => {
                join_values(&mut lhs.value, rhs.value)
            }
            (Reserved, Reserved) | (Unset, Unset | Value(_)) | (Unavailable, _) => Ok(()),
            (slot @ Value(_), Unset) => {
                *slot = Unset;
                Ok(())
            }
            (slot, Reserved | Unavailable | Value(_)) | (slot @ Reserved, _) => {
                *slot = Unavailable;
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LocalVariables {
    slots: Box<[LocalSlot]>,
}

/// Slots assigned to the method-entry values in a fresh local-variable table.
///
/// This is the observable form of the convention that parameters follow the
/// receiver in descriptor order, with a category-2 parameter occupying two
/// slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntrySlots {
    /// The slot holding the receiver, if the method is an instance method.
    pub(crate) this: Option<u16>,
    /// The slot holding each parameter, in descriptor order.
    pub(crate) parameters: Vec<u16>,
}

impl LocalVariables {
    fn invalidate_overlapping_category_2(&mut self, index: usize) {
        if index > 0
            && matches!(self.slots[index], LocalSlot::Reserved)
            && matches!(
                &self.slots[index - 1],
                LocalSlot::Value(previous) if previous.category == ValueCategory::Category2
            )
        {
            self.slots[index - 1] = LocalSlot::Unavailable;
        }
        if matches!(
            &self.slots[index],
            LocalSlot::Value(previous) if previous.category == ValueCategory::Category2
        ) && matches!(self.slots.get(index + 1), Some(LocalSlot::Reserved))
        {
            self.slots[index + 1] = LocalSlot::Unavailable;
        }
    }

    pub fn get(&self, index: u16, expected: ValueCategory) -> Result<&ValueId, Error> {
        let index = usize::from(index);
        let value = match self.slots.get(index).ok_or(Error::LocalIndexOutOfBounds)? {
            LocalSlot::Value(value) if value.category == expected => &value.value,
            LocalSlot::Value(_) | LocalSlot::Reserved => {
                return Err(Error::InvalidSlotLayout);
            }
            LocalSlot::Unavailable => return Err(Error::UnavailableLocal),
            LocalSlot::Unset => return Err(Error::UninitializedLocal),
        };
        if expected == ValueCategory::Category2
            && !matches!(self.slots.get(index + 1), Some(LocalSlot::Reserved))
        {
            return Err(Error::InvalidSlotLayout);
        }
        Ok(value)
    }

    pub fn set(
        &mut self,
        index: u16,
        value: ValueId,
        category: ValueCategory,
    ) -> Result<(), Error> {
        let index = usize::from(index);
        let end = index
            .checked_add(category.slot_count())
            .ok_or(Error::LocalIndexOutOfBounds)?;
        if end > self.slots.len() {
            return Err(Error::LocalIndexOutOfBounds);
        }

        for overwritten in index..end {
            self.invalidate_overlapping_category_2(overwritten);
        }

        self.slots[index] = LocalSlot::Value(LocalValue { value, category });
        if category == ValueCategory::Category2 {
            self.slots[index + 1] = LocalSlot::Reserved;
        }
        Ok(())
    }

    /// Builds the locals of a method entry frame, also reporting the slot each
    /// entry value was assigned.
    pub(super) fn for_method_entry(
        descriptor: &MethodDescriptor,
        max_slots: u16,
        this_value: Option<ValueId>,
        parameters: &[ValueId],
    ) -> Result<(Self, EntrySlots), Error> {
        if parameters.len() != descriptor.parameters_types.len() {
            return Err(Error::ParameterCountMismatch);
        }
        let mut locals = Self {
            slots: vec![LocalSlot::Unset; max_slots.into()].into_boxed_slice(),
        };
        let mut index = 0;
        let this = if let Some(this_value) = this_value {
            locals.set(index, this_value, ValueCategory::Category1)?;
            let slot = index;
            index += 1;
            Some(slot)
        } else {
            None
        };
        let mut parameter_slots = Vec::with_capacity(parameters.len());
        for (value_type, value) in descriptor.parameters_types.iter().zip(parameters) {
            let category = ValueCategory::of_field_type(value_type);
            locals.set(index, *value, category)?;
            parameter_slots.push(index);
            index += u16::try_from(category.slot_count())
                .expect("JVM categories occupy at most two slots");
        }
        Ok((
            locals,
            EntrySlots {
                this,
                parameters: parameter_slots,
            },
        ))
    }

    pub(super) fn clear_for_unwind(&mut self) {
        for slot in &mut self.slots {
            if matches!(slot, LocalSlot::Value(_) | LocalSlot::Unset) {
                *slot = LocalSlot::Unset;
            }
        }
    }

    pub(super) fn merge_from_with<E>(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(usize, &mut ValueId, ValueId) -> Result<(), E>,
    ) -> Result<(), E> {
        for (index, (lhs, rhs)) in self.slots.iter_mut().zip(other.slots).enumerate() {
            lhs.merge_from_with(rhs, |lhs, rhs| merge_values(index, lhs, rhs))?;
        }
        Ok(())
    }

    pub(super) fn has_same_shape(&self, other: &Self) -> bool {
        self.slots.len() == other.slots.len()
    }

    pub(super) fn slot_values(&self) -> impl Iterator<Item = Option<&ValueId>> {
        self.slots.iter().map(LocalSlot::value)
    }
}
