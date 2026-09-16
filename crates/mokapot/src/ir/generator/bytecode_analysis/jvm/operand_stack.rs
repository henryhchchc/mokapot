use std::iter::{once, repeat_n};

use itertools::Itertools;

use super::{ValueCategory, error::Error};
use crate::types::method_descriptor::MethodDescriptor;

use ValueCategory::{Category1, Category2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackOperation {
    Pop,
    Pop2,
    Dup,
    DupX1,
    DupX2,
    Dup2,
    Dup2X1,
    Dup2X2,
    Swap,
}

impl StackOperation {
    const fn consumed_slots(self) -> usize {
        match self {
            Self::Pop | Self::Dup => 1,
            Self::Pop2 | Self::DupX1 | Self::Dup2 | Self::Swap => 2,
            Self::DupX2 | Self::Dup2X1 => 3,
            Self::Dup2X2 => 4,
        }
    }

    fn matching_form(self, categories: &[ValueCategory]) -> Option<(usize, &'static [usize])> {
        let form = match self {
            Self::Pop if categories.ends_with(&[Category1]) => (1, &[0; 0][..]),
            Self::Pop2 if categories.ends_with(&[Category2]) => (1, &[0; 0][..]),
            Self::Pop2 if categories.ends_with(&[Category1, Category1]) => (2, &[0; 0][..]),
            Self::Dup if categories.ends_with(&[Category1]) => (1, &[0, 0][..]),
            Self::DupX1 if categories.ends_with(&[Category1, Category1]) => (2, &[1, 0, 1][..]),
            Self::DupX2 if categories.ends_with(&[Category2, Category1]) => (2, &[1, 0, 1][..]),
            Self::DupX2 if categories.ends_with(&[Category1, Category1, Category1]) => {
                (3, &[2, 0, 1, 2][..])
            }
            Self::Dup2 if categories.ends_with(&[Category2]) => (1, &[0, 0][..]),
            Self::Dup2 if categories.ends_with(&[Category1, Category1]) => (2, &[0, 1, 0, 1][..]),
            Self::Dup2X1 if categories.ends_with(&[Category1, Category2]) => (2, &[1, 0, 1][..]),
            Self::Dup2X1 if categories.ends_with(&[Category1, Category1, Category1]) => {
                (3, &[1, 2, 0, 1, 2][..])
            }
            Self::Dup2X2 if categories.ends_with(&[Category2, Category2]) => (2, &[1, 0, 1][..]),
            Self::Dup2X2 if categories.ends_with(&[Category1, Category1, Category2]) => {
                (3, &[2, 0, 1, 2][..])
            }
            Self::Dup2X2 if categories.ends_with(&[Category2, Category1, Category1]) => {
                (3, &[1, 2, 0, 1, 2][..])
            }
            Self::Dup2X2 if categories.ends_with(&[Category1, Category1, Category1, Category1]) => {
                (4, &[2, 3, 0, 1, 2, 3][..])
            }
            Self::Swap if categories.ends_with(&[Category1, Category1]) => (2, &[1, 0][..]),
            _ => return None,
        };
        Some(form)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct StackItem<V> {
    value: V,
    category: ValueCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OperandStack<V> {
    max_slots: u16,
    slot_count: usize,
    values: Vec<StackItem<V>>,
}

impl<V> OperandStack<V> {
    pub(super) fn with_max_slots(max_slots: u16) -> Self {
        Self {
            max_slots,
            slot_count: 0,
            values: Vec::with_capacity(max_slots.into()),
        }
    }

    pub fn push(&mut self, value: V, category: ValueCategory) -> Result<(), Error> {
        let slot_count = self.slot_count + category.slot_count();
        if slot_count > usize::from(self.max_slots) {
            return Err(Error::StackOverflow);
        }
        self.values.push(StackItem { value, category });
        self.slot_count = slot_count;
        Ok(())
    }

    pub fn pop(&mut self, expected: ValueCategory) -> Result<V, Error> {
        let top = self.values.last().ok_or(Error::StackUnderflow)?;
        if top.category != expected {
            return Err(Error::InvalidSlotLayout);
        }
        let value = self.values.pop().expect("stack was checked as non-empty");
        self.slot_count -= expected.slot_count();
        Ok(value.value)
    }

    pub fn pop_arguments(&mut self, descriptor: &MethodDescriptor) -> Result<Vec<V>, Error> {
        let mut arguments: Vec<_> = descriptor
            .parameters_types
            .iter()
            .rev()
            .map(|value_type| self.pop(ValueCategory::of_field_type(value_type)))
            .try_collect()?;
        arguments.reverse();
        Ok(arguments)
    }

    pub fn apply(&mut self, operation: StackOperation) -> Result<(), Error>
    where
        V: Clone,
    {
        if self.slot_count < operation.consumed_slots() {
            return Err(Error::StackUnderflow);
        }
        let categories = self
            .values
            .iter()
            .map(|value| value.category)
            .collect::<Vec<_>>();
        let (input_len, output_indices) = operation
            .matching_form(&categories)
            .ok_or(Error::InvalidSlotLayout)?;
        let input_start = self.values.len() - input_len;
        let output_slots = output_indices
            .iter()
            .map(|index| self.values[input_start + index].category.slot_count())
            .sum::<usize>();
        let resulting_slots = self.slot_count - operation.consumed_slots() + output_slots;
        if resulting_slots > usize::from(self.max_slots) {
            return Err(Error::StackOverflow);
        }

        let output = output_indices
            .iter()
            .map(|index| self.values[input_start + index].clone())
            .collect::<Vec<_>>();
        self.values.truncate(input_start);
        self.values.extend(output);
        self.slot_count = resulting_slots;
        Ok(())
    }

    pub(super) const fn max_slots(&self) -> u16 {
        self.max_slots
    }

    pub(super) fn clear(&mut self) {
        self.values.clear();
        self.slot_count = 0;
    }

    pub(super) fn has_same_shape(&self, other: &Self) -> bool {
        self.max_slots == other.max_slots
            && self
                .values
                .iter()
                .map(|value| value.category)
                .eq(other.values.iter().map(|value| value.category))
    }

    pub(super) fn merge_from_with(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(usize, &mut V, V) -> bool,
    ) -> bool {
        let mut slot = 0;
        self.values
            .iter_mut()
            .zip(other.values)
            .fold(false, |changed, (lhs, rhs)| {
                slot += lhs.category.slot_count() - 1;
                let value_changed = merge_values(slot, &mut lhs.value, rhs.value);
                slot += 1;
                value_changed || changed
            })
    }

    pub(super) fn single_value(&self, expected: ValueCategory) -> Result<&V, Error> {
        let [value] = self.values.as_slice() else {
            return Err(Error::InvalidSlotLayout);
        };
        if value.category != expected {
            return Err(Error::InvalidSlotLayout);
        }
        Ok(&value.value)
    }

    pub(super) fn slot_values(&self) -> impl Iterator<Item = Option<&V>> {
        self.values.iter().flat_map(|it| {
            repeat_n(None, it.category.slot_count() - 1).chain(once(Some(&it.value)))
        })
    }
}
