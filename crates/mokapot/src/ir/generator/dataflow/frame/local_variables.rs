use super::error::Error;
use crate::{
    ir::ValueId,
    types::{field_type::ValueCategory, method_descriptor::MethodDescriptor},
};

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

    fn merge_from_with(&mut self, other: Self, join_values: impl FnOnce(&mut ValueId, ValueId)) {
        use LocalSlot::{Reserved, Unavailable, Unset, Value};
        match (self, other) {
            (Value(lhs), Value(rhs)) if lhs.category == rhs.category => {
                join_values(&mut lhs.value, rhs.value);
            }
            (Reserved, Reserved) | (Unset, Unset | Value(_)) | (Unavailable, _) => {}
            (slot @ Value(_), Unset) => *slot = Unset,
            (slot, Reserved | Unavailable | Value(_)) | (slot @ Reserved, _) => *slot = Unavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LocalVariables {
    slots: Box<[LocalSlot]>,
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

    pub(crate) fn get(&self, index: u16, expected: ValueCategory) -> Result<&ValueId, Error> {
        let index = usize::from(index);
        match self.slots.get(index).ok_or(Error::LocalIndexOutOfBounds)? {
            LocalSlot::Value(value) if value.category == expected => Ok(&value.value),
            LocalSlot::Value(_) | LocalSlot::Reserved => Err(Error::InvalidSlotLayout),
            LocalSlot::Unavailable => Err(Error::UnavailableLocal),
            LocalSlot::Unset => Err(Error::UninitializedLocal),
        }
    }

    pub(crate) fn set(
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

    /// Builds the locals of a method entry frame.
    pub(super) fn for_method_entry(
        descriptor: &MethodDescriptor,
        max_slots: u16,
        this_value: Option<ValueId>,
        parameters: &[ValueId],
    ) -> Result<Self, Error> {
        if parameters.len() != descriptor.parameters_types.len() {
            return Err(Error::ParameterCountMismatch);
        }
        let mut locals = Self {
            slots: vec![LocalSlot::Unset; max_slots.into()].into_boxed_slice(),
        };
        let params = descriptor
            .parameters_types
            .iter()
            .zip(parameters)
            .map(|(ty, value)| (*value, ty.value_category()));
        let mut entries = this_value
            .map(|value| (value, ValueCategory::Category1))
            .into_iter()
            .chain(params);
        entries.try_fold(0_u16, |index, (value, category)| {
            locals.set(index, value, category)?;
            let slots = u16::try_from(category.slot_count())
                .expect("JVM categories occupy at most two slots");
            Ok(index + slots)
        })?;
        Ok(locals)
    }

    pub(super) fn merge_from_with(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(usize, &mut ValueId, ValueId),
    ) {
        for (index, (lhs, rhs)) in self.slots.iter_mut().zip(other.slots).enumerate() {
            lhs.merge_from_with(rhs, |lhs, rhs| merge_values(index, lhs, rhs));
        }
    }

    pub(super) fn has_same_shape(&self, other: &Self) -> bool {
        self.slots.len() == other.slots.len()
    }

    pub(super) fn slot_values(&self) -> impl Iterator<Item = Option<&ValueId>> {
        self.slots.iter().map(LocalSlot::value)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{Error, LocalSlot, LocalVariables};
    use crate::{
        ir::{IdAllocator, ValueId},
        types::field_type::ValueCategory,
    };
    use ValueCategory::{Category1, Category2};

    /// A table of `slot_count` variables, none of them written. Built from primitives rather than a
    /// derived `Arbitrary`, which would not keep a category 2 value's upper variable reserved.
    fn empty_locals(slot_count: usize) -> LocalVariables {
        LocalVariables {
            slots: vec![LocalSlot::Unset; slot_count].into_boxed_slice(),
        }
    }

    proptest! {
        /// A store reads back under its own category and, for a category 2 value, reserves the
        /// variable above it, from which nothing can be loaded (JVMS §2.6.1).
        #[test]
        fn a_store_reads_back_and_reserves_the_variable_above_it(
            slot_count in 1..6_usize,
            index in 0..8_u16,
            value in any::<ValueId>(),
            category in any::<ValueCategory>(),
        ) {
            let mut locals = empty_locals(slot_count);
            match locals.set(index, value, category) {
                Ok(()) => {
                    prop_assert_eq!(locals.get(index, category), Ok(&value));
                    if category == Category2 {
                        let reserved = index + 1;
                        prop_assert_eq!(
                            locals.get(reserved, Category1),
                            Err(Error::InvalidSlotLayout),
                        );
                        prop_assert_eq!(
                            locals.get(reserved, Category2),
                            Err(Error::InvalidSlotLayout),
                        );
                    }
                }
                // Only an address the value does not fit in is refused.
                Err(error) => prop_assert_eq!(error, Error::LocalIndexOutOfBounds),
            }
        }

        /// Overwriting either half of a category 2 value never leaves the original readable, not even
        /// when the store installs a fresh pair there (JVMS §4.10.2.3).
        #[test]
        fn overwriting_a_half_of_a_category_2_value_never_leaves_it_readable(
            start in 0..3_u16,
            category in any::<ValueCategory>(),
            higher_half in any::<bool>(),
        ) {
            let mut ids = IdAllocator::default();
            let value = ids.new_id();
            let overwrite = ids.new_id();
            // A table long enough for the pair at `start` and a two-slot overwrite of either half.
            let mut locals = empty_locals(6);
            locals.set(start, value, Category2).expect("the pair fits");
            prop_assert_eq!(locals.get(start, Category2), Ok(&value));

            let overwritten = if higher_half { start + 1 } else { start };
            locals.set(overwritten, overwrite, category).expect("the overwrite fits");
            let read = locals.get(start, Category2);

            // The failure is the implementation's taxonomy; that the pair stops reading is not.
            if category == Category1 || higher_half {
                prop_assert!(read.is_err(), "the overwritten pair still reads back");
            }
            prop_assert_ne!(read, Ok(&value), "the old value still reads back");
            prop_assert_eq!(locals.get(overwritten, category), Ok(&overwrite));
        }

        /// A read of a variable that was never written, and of a value under the category the
        /// variable does not hold, is refused.
        #[test]
        fn illegal_accesses_are_refused(
            slot_count in 0..6_usize,
            value in any::<ValueId>(),
            category in any::<ValueCategory>(),
        ) {
            let locals = empty_locals(slot_count);
            for slot in 0..slot_count {
                let index = u16::try_from(slot).expect("the small tables fit in u16");
                prop_assert_eq!(locals.get(index, category), Err(Error::UninitializedLocal));
            }

            let mut locals = empty_locals(2);
            locals.set(0, value, category).expect("two variables hold any value");
            let other = match category {
                Category1 => Category2,
                Category2 => Category1,
            };
            prop_assert_eq!(locals.get(0, other), Err(Error::InvalidSlotLayout));
        }
    }
}
