use super::{ValueCategory, error::Error};
use crate::types::method_descriptor::MethodDescriptor;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct LocalValue<V> {
    value: V,
    category: ValueCategory,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
enum LocalSlot<V> {
    Value(LocalValue<V>),
    Reserved,
    Unset,
    Unavailable,
}

impl<V> LocalSlot<V> {
    const fn value(&self) -> Option<&V> {
        match self {
            Self::Value(value) => Some(&value.value),
            Self::Reserved | Self::Unset | Self::Unavailable => None,
        }
    }

    fn merge_from_with(
        &mut self,
        other: Self,
        join_values: impl FnOnce(&mut V, V) -> bool,
    ) -> bool {
        use LocalSlot::{Reserved, Unavailable, Unset, Value};
        match (self, other) {
            (Value(lhs), Value(rhs)) if lhs.category == rhs.category => {
                join_values(&mut lhs.value, rhs.value)
            }
            (Reserved, Reserved) | (Unset, Unset | Value(_)) | (Unavailable, _) => false,
            (slot @ Value(_), Unset) => {
                *slot = Unset;
                true
            }
            (slot, Reserved | Unavailable | Value(_)) | (slot @ Reserved, _) => {
                *slot = Unavailable;
                true
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LocalVariables<V> {
    slots: Box<[LocalSlot<V>]>,
}

impl<V> LocalVariables<V> {
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

    pub fn get(&self, index: u16, expected: ValueCategory) -> Result<&V, Error> {
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

    pub fn set(&mut self, index: u16, value: V, category: ValueCategory) -> Result<(), Error> {
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

    pub(super) fn for_method_entry(
        descriptor: &MethodDescriptor,
        max_slots: u16,
        this_value: Option<V>,
        parameters: &[V],
    ) -> Result<Self, Error>
    where
        V: Clone,
    {
        if parameters.len() != descriptor.parameters_types.len() {
            return Err(Error::ParameterCountMismatch);
        }
        let mut locals = Self {
            slots: vec![LocalSlot::Unset; max_slots.into()].into_boxed_slice(),
        };
        let mut index = 0;
        if let Some(this_value) = this_value {
            locals.set(index, this_value, ValueCategory::Category1)?;
            index += 1;
        }
        for (value_type, value) in descriptor.parameters_types.iter().zip(parameters) {
            let category = ValueCategory::of_field_type(value_type);
            locals.set(index, value.clone(), category)?;
            index += u16::try_from(category.slot_count())
                .expect("JVM categories occupy at most two slots");
        }
        Ok(locals)
    }

    pub(super) fn clear_for_unwind(&mut self) {
        for slot in &mut self.slots {
            if matches!(slot, LocalSlot::Value(_) | LocalSlot::Unset) {
                *slot = LocalSlot::Unset;
            }
        }
    }

    pub(super) fn merge_from_with(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(usize, &mut V, V) -> bool,
    ) -> bool {
        self.slots.iter_mut().zip(other.slots).enumerate().fold(
            false,
            |changed, (index, (lhs, rhs))| {
                lhs.merge_from_with(rhs, |lhs, rhs| merge_values(index, lhs, rhs)) || changed
            },
        )
    }

    pub(super) fn has_same_shape(&self, other: &Self) -> bool {
        self.slots.len() == other.slots.len()
    }

    pub(super) fn slot_values(&self) -> impl Iterator<Item = Option<&V>> {
        self.slots.iter().map(LocalSlot::value)
    }
}
